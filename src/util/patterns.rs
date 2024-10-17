use super::{err_or_ignore, increment, msg, Log};
use crate::{Cfg, ListOptions, RegexPluginInfo};
use anyhow::{Context, Result};
use fs_err::{metadata, read_dir};
use glob::{glob_with, MatchOptions};
use regex::RegexBuilder;
use std::{
    fmt::Write as _,
    path::{Path, MAIN_SEPARATOR},
    time::SystemTime,
};

pub fn get_regex_plugin_list(
    plugin_list: &[String],
    index: usize,
    list_options: &ListOptions,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<Vec<String>> {
    let mut regex_plugin_list = Vec::new();
    let regex_sublists = get_regex_sublists(plugin_list, index, list_options, cfg, log)?;
    if !regex_sublists.is_empty() {
        regex_plugin_list = plugin_list
            .get(..index)
            .with_context(|| format!("Bug: indexing slicing plugin_list[..{index}]"))?
            .iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let mut new_index = index;
        let mut sum: usize;
        for (subindex, sublist) in regex_sublists {
            sum = subindex.checked_add(index).with_context(|| {
                format!("Bug: overflow adding index = \"{index}\" to subindex = \"{subindex}\"")
            })?;
            if sum > new_index {
                regex_plugin_list.extend(
                    plugin_list
                        .get(new_index..sum)
                        .with_context(|| {
                            format!("Bug: indexing slicing plugin_list[{new_index}..{sum}]")
                        })?
                        .iter()
                        .map(ToOwned::to_owned),
                );
                new_index = sum;
            }
            if !sublist.is_empty() {
                regex_plugin_list.extend(sublist.into_iter());
            }
            new_index = increment!(new_index);
        }
        if new_index < plugin_list.len() {
            regex_plugin_list.extend(
                plugin_list
                    .get(new_index..)
                    .with_context(|| format!("Bug: indexing slicing plugin_list[{new_index}..]"))?
                    .iter()
                    .map(ToOwned::to_owned),
            );
        }
    }
    Ok(regex_plugin_list)
}

fn get_regex_sublists(
    plugin_list: &[String],
    index: usize,
    list_options: &ListOptions,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<Vec<(usize, Vec<String>)>> {
    let mut regex_sublists = Vec::new();
    let mut sublist = Vec::new();
    let mut split: Vec<&str>;
    for (subindex, item) in plugin_list
        .get(index..)
        .with_context(|| format!("Bug: indexing slicing plugin_list[{index}..]"))?
        .iter()
        .enumerate()
    {
        split = item.splitn(2, ':').collect();
        if split.len() != 2 {
            continue;
        }
        #[allow(clippy::indexing_slicing)]
        let (pattern, is_regex) = if split[0].to_lowercase() == "regex" {
            (split[1], true)
        } else if split[0].to_lowercase() == "glob" {
            (split[1], false)
        } else {
            continue;
        };
        if pattern.is_empty() {
            let text = format!("Pattern is empty in argument: {item:?}");
            err_or_ignore(text, list_options.ignore_important_errors, false, cfg, log)?;
            regex_sublists.push((subindex, Vec::new()));
            continue;
        }
        let mut sort_by_name = list_options.regex_sort_by_name;
        let plugin_pathbuf = list_options.indirect.base_dir.join(pattern);
        let mut remove_leading_dot = false;
        sublist.clear();
        if let Err(error) = if is_regex {
            get_regex_plugins(
                &mut sublist,
                &plugin_pathbuf,
                &mut sort_by_name,
                &mut remove_leading_dot,
                list_options,
                cfg,
                log,
            )
        } else {
            get_glob_plugins(
                &mut sublist,
                &plugin_pathbuf,
                &mut sort_by_name,
                list_options,
                cfg,
                log,
            )
        } {
            err_or_ignore(
                format!("{error:?}"),
                list_options.ignore_important_errors,
                false,
                cfg,
                log,
            )
            .with_context(|| {
                format!(
                    "Failed to get plugins from {} pattern: {pattern:?}",
                    if is_regex { "regex" } else { "glob" }
                )
            })?;
            regex_sublists.push((subindex, Vec::new()));
            continue;
        };
        if sort_by_name {
            sublist.sort_by(|a, b| a.name_low.cmp(&b.name_low).then(a.path.cmp(&b.path)));
        } else {
            sublist.sort_by(|a, b| a.time.cmp(&b.time).then(a.path.cmp(&b.path)));
        }
        let regex_sublist = get_regex_sublist(&sublist, remove_leading_dot, list_options);
        if regex_sublist.is_empty() {
            let text = format!("Nothing found for pattern: {pattern:?}");
            err_or_ignore(text, list_options.ignore_important_errors, false, cfg, log)?;
        } else {
            let mut text = format!("Pattern {item:?} expanded to:");
            for plugin in &regex_sublist {
                if plugin.contains(' ') {
                    write!(text, " \"{plugin}\"")?;
                } else {
                    write!(text, " {plugin}")?;
                };
            }
            msg(&text, 0, cfg, log)?;
        }
        regex_sublists.push((subindex, regex_sublist));
    }
    Ok(regex_sublists)
}

fn get_regex_sublist(
    sublist: &[RegexPluginInfo],
    remove_leading_dot: bool,
    list_options: &ListOptions,
) -> Vec<String> {
    let prefix = if remove_leading_dot {
        format!(".{MAIN_SEPARATOR}")
    } else if !list_options.indirect.base_dir.as_os_str().is_empty() {
        format!(
            "{}{MAIN_SEPARATOR}",
            list_options.indirect.base_dir.to_string_lossy()
        )
    } else {
        String::new()
    };
    sublist
        .iter()
        .map(|regex_plugin_info| regex_plugin_info.path.to_string_lossy())
        .map(|path_str| {
            if prefix.is_empty() {
                path_str.into_owned()
            } else if let Some(stripped) = path_str.strip_prefix(&prefix) {
                stripped.to_owned()
            } else {
                path_str.into_owned()
            }
        })
        .collect::<Vec<String>>()
}

fn get_plugin_time(
    path: &Path,
    sort_by_name: &mut bool,
    pattern: &Path,
    pattern_kind: &str,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<SystemTime> {
    let time = if *sort_by_name {
        SystemTime::now()
    } else {
        match metadata(path) {
            Ok(meta) => match meta.modified() {
                Ok(time) => time,
                Err(error) => {
                    let text = format!(
                    "Falling back to \"--sort-by-name\" for the {pattern_kind} {pattern:?} because failed to get file modification time for {path:?} with error: {error:?}"
                );
                    msg(&text, 0, cfg, log)?;
                    *sort_by_name = false;
                    SystemTime::now()
                }
            },
            Err(error) => {
                let text = format!(
                    "Falling back to \"--sort-by-name\" for the {pattern_kind} {pattern:?} because failed to get file metadata for {path:?} with error: {error:?}"
                );
                msg(&text, 0, cfg, log)?;
                *sort_by_name = false;
                SystemTime::now()
            }
        }
    };
    Ok(time)
}

fn get_regex_plugins(
    list: &mut Vec<RegexPluginInfo>,
    pattern: &Path,
    sort_by_name: &mut bool,
    remove_leading_dot: &mut bool,
    list_options: &ListOptions,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let mut dir = Path::new(&pattern);
    loop {
        match dir.parent() {
            None => break,
            Some(parent) => {
                dir = parent;
                if parent.is_dir() {
                    break;
                }
            }
        }
    }
    let regex_pattern = pattern
        .to_string_lossy()
        .strip_prefix(&format!("{}{}", dir.to_string_lossy(), MAIN_SEPARATOR))
        .map_or_else(|| pattern.to_string_lossy().into_owned(), ToOwned::to_owned);
    let regex_expression = RegexBuilder::new(&regex_pattern)
        .case_insensitive(!list_options.regex_case_sensitive)
        .build()?;
    if dir == Path::new("") {
        *remove_leading_dot = true;
        dir = Path::new(".");
    };
    for entry in read_dir(dir)?.flatten() {
        if entry
            .file_type()
            .map_or(true, |file_type| !file_type.is_dir())
        {
            let path = entry.path();
            if let Some(plugin_extension) = path.extension() {
                if cfg
                    .guts
                    .omw_plugin_extensions
                    .contains(&plugin_extension.to_ascii_lowercase())
                    && regex_expression.is_match(&entry.file_name().to_string_lossy())
                {
                    let time = get_plugin_time(&path, sort_by_name, pattern, "regex", cfg, log)
                        .with_context(|| {
                            format!("Failed to get modification time for: {path:?}")
                        })?;
                    let name_low = entry.file_name().to_string_lossy().to_lowercase();
                    list.push(RegexPluginInfo {
                        path,
                        name_low,
                        time,
                    });
                }
            }
        }
    }
    Ok(())
}

fn get_glob_plugins(
    list: &mut Vec<RegexPluginInfo>,
    pattern: &Path,
    sort_by_name: &mut bool,
    list_options: &ListOptions,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let glob_options = MatchOptions {
        case_sensitive: list_options.regex_case_sensitive,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };
    for path in glob_with(&pattern.to_string_lossy(), glob_options)?.flatten() {
        let name_low = match path.file_name() {
            Some(osstr) => osstr.to_string_lossy().to_lowercase(),
            None => continue,
        };
        if let Some(plugin_extension) = path.extension() {
            if cfg
                .guts
                .omw_plugin_extensions
                .contains(&plugin_extension.to_ascii_lowercase())
            {
                let time = get_plugin_time(&path, sort_by_name, pattern, "glob", cfg, log)
                    .with_context(|| format!("Failed to get modification time for: {path:?}"))?;
                list.push(RegexPluginInfo {
                    path,
                    name_low,
                    time,
                });
            }
        };
    }
    Ok(())
}
