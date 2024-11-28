use super::{GlobalGetPluginsHelper, LocalGetPluginsHelper};
use crate::{err_or_ignore, err_or_ignore_thread_safe, increment, msg, Cfg, LoadOrder, Log};
use anyhow::{anyhow, Context as _, Result};
use dirs::{data_dir, document_dir};
use fs_err::read_dir;
use hashbrown::{hash_map::Entry, HashMap};
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use std::{fmt::Write as _, path::PathBuf};

pub(super) fn get_all_plugins(
    omw_data_dirs: &[(usize, PathBuf)],
    lcl_h: &mut LocalGetPluginsHelper,
    glb_h: &GlobalGetPluginsHelper,
    cfg: &Cfg,
) -> Result<HashMap<String, PathBuf>> {
    let mut found_plugins: Vec<(usize, String, PathBuf)> = omw_data_dirs
        .par_iter()
        .map(
            |&(id, ref dir_path)| -> Result<Vec<(usize, String, PathBuf)>, _> {
                let mut res: Vec<(usize, String, PathBuf)> = Vec::new();
                match read_dir(dir_path) {
                    Ok(dir_contents) => {
                        for entry in dir_contents.flatten() {
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
                                    {
                                        res.push((
                                            id,
                                            entry.file_name().to_string_lossy().into_owned(),
                                            path,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    Err(error) => {
                        let text = format!(
                            "Failed to open directory \"{}\" with error: \"{:#}\"",
                            dir_path.display(),
                            error
                        );
                        err_or_ignore_thread_safe(text, glb_h.ignore, cfg)?;
                    }
                }

                Ok(res)
            },
        )
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|vec| !vec.is_empty())
        .flatten()
        .collect();
    found_plugins.sort();
    let mut all_plugins: HashMap<String, PathBuf> = HashMap::new();
    found_plugins
        .into_iter()
        .rev()
        .for_each(|(_, plugin, path)| {
            if let Entry::Vacant(v) = all_plugins.entry(plugin) {
                v.insert(path);
            }
        });
    lcl_h.omw_all_plugins_found = true;
    Ok(all_plugins)
}

pub(super) fn get_data_dir(
    line: &str,
    omw_data_dirs: &mut Vec<(usize, PathBuf)>,
    lcl_h: &mut LocalGetPluginsHelper,
    glb_h: &GlobalGetPluginsHelper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    if let Some(raw_data) = line.split('=').nth(1) {
        let data = PathBuf::from(if raw_data.starts_with('"') && raw_data.ends_with('"') {
            raw_data
                .get(
                    1..raw_data.len().checked_sub(1).with_context(|| {
                        format!(
                            "Bug: overflow decrementing raw_data.len() = \"{}\"",
                            raw_data.len()
                        )
                    })?,
                )
                .with_context(|| format!("Bug: indexing slicing raw_data[1..{}]", raw_data.len()))?
        } else {
            raw_data
        });
        omw_data_dirs.push((lcl_h.omw_data_counter, data));
        lcl_h.omw_data_counter = increment!(lcl_h.omw_data_counter);
    } else {
        let text = format!("Failed to parse line \"{line}\"");
        err_or_ignore(text, glb_h.ignore, false, cfg, log)?;
    }
    Ok(())
}

pub(super) fn push_line_ending(
    vec: &mut Vec<String>,
    line: &str,
    lcl_h: &mut LocalGetPluginsHelper,
) {
    lcl_h.set_omw_found();
    if let Some(raw_name) = line.split('=').nth(1) {
        vec.push(raw_name.trim().to_owned());
    }
}

pub(super) fn get_plugin(
    name: &String,
    load_order: &mut LoadOrder,
    omw_all_plugins: &HashMap<String, PathBuf>,
    kind: &str,
    glb_h: &GlobalGetPluginsHelper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    if let Some(path) = omw_all_plugins.get(name) {
        match kind {
            "plugin" => load_order
                .contents
                .push(path.to_string_lossy().into_owned()),
            "groundcover" => load_order
                .groundcovers
                .push(path.to_string_lossy().into_owned()),
            "fallback archive" => {
                let modification_time = path.metadata().map_or(None, |meta| meta.modified().ok());
                load_order.fallback_archives.push((
                    load_order.fallback_archives.len(),
                    path.to_string_lossy().into_owned(),
                    modification_time,
                ));
            }
            _ => {
                return Err(anyhow!(
                    "Wrong argument passed to the function \"omw_get_plugin\""
                ))
            }
        }
    } else {
        let text = format!("Failed to find {kind} \"{name}\"");
        err_or_ignore(text, glb_h.ignore, false, cfg, log)?;
    }
    Ok(())
}

pub(super) fn get_cs_data_dir(
    omw_data_dirs: &mut Vec<(usize, PathBuf)>,
    lcl_h: &mut LocalGetPluginsHelper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let mut checked_paths: Vec<PathBuf> = Vec::new();
    macro_rules! check_omw_cs_data_path {
        ($omw_cs_data_path:expr) => {
            if $omw_cs_data_path.exists() {
                omw_data_dirs.push((lcl_h.omw_data_counter, $omw_cs_data_path));
                lcl_h.omw_data_counter = increment!(lcl_h.omw_data_counter);
                let text = format!(
                    "Added \"hidden\" OpenMW-CS data path \"{}\" to the list of directories",
                    $omw_cs_data_path.display()
                );
                return msg(text, 0, cfg, log);
            }
            checked_paths.push($omw_cs_data_path);
        };
    }
    if let Some(dir) = data_dir() {
        check_omw_cs_data_path!(dir.join(&cfg.guts.omw_cs_data_path_suffix_linux_macos));
    } else {
        checked_paths.push(PathBuf::from(format!(
            "Failed to get __data_dir__ to check \"__data_dir__/{}\"",
            &cfg.guts.omw_cs_data_path_suffix_linux_macos
        )));
    };
    if let Some(dir) = document_dir() {
        check_omw_cs_data_path!(dir.join(&cfg.guts.omw_cs_data_path_suffix_windows));
    } else {
        checked_paths.push(PathBuf::from(format!(
            "Failed to get __document_dir__ to check \"__document_dir__/{}\"",
            &cfg.guts.omw_cs_data_path_suffix_windows
        )));
    };
    for path in &cfg.guts.omw_cs_data_paths_list {
        check_omw_cs_data_path!(PathBuf::from(path));
    }
    let text = format!(
        "Failed to find \"hidden\" OpenMW-CS data path. Probably none exists. Checked following paths:\n{}",
        checked_paths
            .iter()
            .map(|path| format!("\t{}", path.display()))
            .collect::<Vec<String>>()
            .join("\n")
    );
    msg(text, 1, cfg, log)
}

pub(super) fn msg_force_base_dir(
    text: &mut String,
    lcl_h: &LocalGetPluginsHelper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    text.clear();
    match lcl_h.omw_data_line_counter {
        0 => {}
        1 => {
            writeln!(
                text,
                "Ignored \"{}\" line due to \"force_base_dir\" flag",
                &cfg.guts.omw_line_beginning_data,
            )?;
        }
        _ => {
            writeln!(
                text,
                "Ignored all {} \"{}\" lines due to \"force_base_dir\" flag",
                lcl_h.omw_data_line_counter, &cfg.guts.omw_line_beginning_data,
            )?;
        }
    };
    write!(
        text,
        "Skipped search of \"hidden\" OpenMW-CS data path due to \"force_base_dir\" flag"
    )?;
    msg(text, 1, cfg, log)
}

pub(super) fn msg_no_data_lines(text: &mut String, cfg: &Cfg, log: &mut Log) -> Result<()> {
    text.clear();
    write!(
        text,
        "Warning: game configuration file doesn't contain \"{}\" lines",
        &cfg.guts.omw_line_beginning_data,
    )?;
    msg(&text, 0, cfg, log)
}

pub(super) fn msg_no_data_dirs(text: &mut String, cfg: &Cfg, log: &mut Log) -> Result<()> {
    text.clear();
    write!(
        text,
        "Failed to get any \"{}\" directory: falling back to current directory to look for plugins",
        &cfg.guts.omw_line_beginning_data,
    )?;
    msg(&text, 0, cfg, log)
}
