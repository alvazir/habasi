use crate::{
    err_or_ignore, err_or_ignore_thread_safe, msg, read_lines, Cfg, Helper, LoadOrder, Log,
};
use anyhow::{anyhow, Context, Result};
use dirs::{data_dir, document_dir};
use fs_err::read_dir;
use hashbrown::{hash_map::Entry, HashMap};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::path::{Path, PathBuf};
mod game_config;

pub fn scan(h: &mut Helper, cfg: &Cfg, log: &mut Log) -> Result<()> {
    if h.g.config_index == usize::MAX {
        game_config::get(h, cfg, log).with_context(|| "Failed to get game configuration file")?;
    }
    if !h
        .t
        .game_configs
        .get(h.g.config_index)
        .with_context(|| {
            format!(
                "Bug: h.t.game_configs doesn't contain h.g.config_index = \"{}\"",
                h.g.config_index
            )
        })?
        .load_order
        .scanned
    {
        get_load_order(h, cfg, log).with_context(|| "Failed to get load order")?;
    }
    Ok(())
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Default)]
struct GetPluginsHelper {
    mor_found: bool,
    omw_found: bool,
    omw_data_counter: usize,
    mor_data_files_dir: PathBuf,
    mor_data_files_dir_found: bool,
    omw_all_plugins_found: bool,
}

#[allow(clippy::too_many_lines)]
fn get_load_order(h: &mut Helper, cfg: &Cfg, log: &mut Log) -> Result<()> {
    let game_config =
        h.t.game_configs
            .get_mut(h.g.config_index)
            .with_context(|| {
                format!(
                    "Bug: indexing slicing h.g.config_index[{}]",
                    h.g.config_index
                )
            })?;
    let config_path = &game_config.path;
    let config_path_canonical = &game_config.path_canonical;
    let text = format!(
        "Gathering plugins from game configuration file \"{}\"",
        config_path.display()
    );
    msg(text, 1, cfg, log)?;
    let config_lines = read_lines(config_path).with_context(|| {
        format!(
            "Failed to read game configuration file \"{}\"",
            config_path.display()
        )
    })?;
    let ignore = h.g.list_options.ignore_important_errors;
    let mut res: LoadOrder = LoadOrder::default();
    let mut helper: GetPluginsHelper = GetPluginsHelper::default();
    let mut omw_data_dirs: Vec<(usize, PathBuf)> = Vec::new();
    let mut omw_plugins: Vec<String> = Vec::new();
    let mut omw_groundcovers: Vec<String> = Vec::new();
    let mut omw_fallback_archives: Vec<String> = Vec::new();
    let mut omw_all_plugins: HashMap<String, PathBuf> = HashMap::new();
    for line in config_lines.map_while(Result::ok) {
        if !helper.omw_found {
            if line.starts_with(&cfg.guts.mor_line_beginning_content) {
                mor_get_plugin(
                    &line,
                    config_path_canonical,
                    &mut res,
                    &mut helper,
                    ignore,
                    cfg,
                    log,
                )
                .with_context(|| "Failed to find Morrowind's plugin")?;
            } else if line.starts_with(&cfg.guts.mor_line_beginning_archive) {
                mor_get_archive(
                    &line,
                    config_path_canonical,
                    &mut res,
                    false,
                    &mut helper,
                    ignore,
                    cfg,
                    log,
                )
                .with_context(|| "Failed to find Morrowind's archive")?;
            } else { //
            }
        }
        if !helper.mor_found {
            if line.starts_with(&cfg.guts.omw_line_beginning_data) {
                omw_get_data_dir(&line, &mut omw_data_dirs, &mut helper, ignore, cfg, log)
                    .with_context(|| "Failed to get OpenMW's data directory")?;
            } else if line.starts_with(&cfg.guts.omw_line_beginning_fallback_archive) {
                if let Some(raw_name) = line.split('=').nth(1) {
                    omw_fallback_archives.push(raw_name.trim().to_owned());
                }
            } else if line.starts_with(&cfg.guts.omw_line_beginning_groundcover) {
                if let Some(raw_name) = line.split('=').nth(1) {
                    omw_groundcovers.push(raw_name.trim().to_owned());
                }
            } else if line.starts_with(&cfg.guts.omw_line_beginning_content) {
                if let Some(raw_name) = line.split('=').nth(1) {
                    omw_plugins.push(raw_name.trim().to_owned());
                }
            } else { //
            }
        }
    }
    if helper.omw_found {
        if !helper.omw_all_plugins_found {
            omw_get_cs_data_dir(&mut omw_data_dirs, &mut helper, cfg, log)
                .with_context(|| "Failed to find \"hidden\" OpenMW-CS data directory path")?;
            omw_all_plugins = get_all_plugins(&omw_data_dirs, &mut helper, ignore, cfg)
                .with_context(|| "Failed to find all OpenMW's plugins")?;
        };
        res.datas = omw_data_dirs;
        omw_plugins.iter().try_for_each(|plugin| -> Result<()> {
            omw_get_plugin(
                plugin,
                &mut res,
                &omw_all_plugins,
                "plugin",
                ignore,
                cfg,
                log,
            )
            .with_context(|| "Failed to find OpenMW's plugin")
        })?;
        omw_groundcovers
            .iter()
            .try_for_each(|groundcover| -> Result<()> {
                omw_get_plugin(
                    groundcover,
                    &mut res,
                    &omw_all_plugins,
                    "groundcover",
                    ignore,
                    cfg,
                    log,
                )
                .with_context(|| "Failed to find OpenMW's groundcover")
            })?;
        omw_fallback_archives
            .iter()
            .try_for_each(|fallback_archive| -> Result<()> {
                omw_get_plugin(
                    fallback_archive,
                    &mut res,
                    &omw_all_plugins,
                    "fallback archive",
                    ignore,
                    cfg,
                    log,
                )
                .with_context(|| "Failed to find OpenMW's fallback archive")
            })?;
    } else if helper.mor_found {
        let missing_bsa = &cfg.guts.mor_line_missing_archive;
        mor_get_archive(
            missing_bsa,
            config_path,
            &mut res,
            true,
            &mut helper,
            ignore,
            cfg,
            log,
        )
        .with_context(|| "Failed to find Morrowind's base archive")?;
    } else { //
    }
    res.scanned = true;
    game_config.load_order = res;
    Ok(())
}

fn get_all_plugins(
    omw_data_dirs: &[(usize, PathBuf)],
    helper: &mut GetPluginsHelper,
    ignore_important_errors: bool,
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
                        err_or_ignore_thread_safe(text, ignore_important_errors, cfg)?;
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
    helper.omw_all_plugins_found = true;
    Ok(all_plugins)
}

fn mor_get_data_files_dir(
    config_path_canonical: &Path,
    res: &mut LoadOrder,
    helper: &mut GetPluginsHelper,
    cfg: &Cfg,
) -> Result<()> {
    helper.mor_data_files_dir = match config_path_canonical.parent() {
        Some(path) => {
            let data_files_dir = Path::new(path).join(&cfg.guts.mor_data_files_dir);
            res.datas.push((0, data_files_dir.clone()));
            data_files_dir
        }
        None => {
            return Err(anyhow!(
                "Failed to find Morrowind's \"Data Files\" directory at expected location \"{}\"",
                &cfg.guts.mor_data_files_dir
            ))
        }
    };
    helper.mor_data_files_dir_found = true;
    Ok(())
}

fn mor_get_plugin(
    line: &str,
    config_path_canonical: &Path,
    res: &mut LoadOrder,
    helper: &mut GetPluginsHelper,
    ignore_important_errors: bool,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    if !helper.mor_data_files_dir_found {
        mor_get_data_files_dir(config_path_canonical, res, helper, cfg)
            .with_context(|| "Failed to find Morrowind's \"Data Files\" directory")?;
    }
    if let Some(raw_name) = line.split('=').nth(1) {
        let name = raw_name.trim();
        let path = helper.mor_data_files_dir.join(name);
        if path.exists() {
            res.contents.push(path.to_string_lossy().into_owned());
        } else {
            let text = format!(
                "Plugin \"{}\" not found at expected location \"{}\"",
                name,
                helper.mor_data_files_dir.join(name).display()
            );
            err_or_ignore(text, ignore_important_errors, false, cfg, log)?;
        }
    } else {
        let text = format!("Failed to parse line \"{line}\"");
        err_or_ignore(text, ignore_important_errors, false, cfg, log)?;
    }
    if !helper.mor_found {
        helper.mor_found = true;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn mor_get_archive(
    line: &str,
    config_path_canonical: &Path,
    res: &mut LoadOrder,
    prepend: bool,
    helper: &mut GetPluginsHelper,
    ignore_important_errors: bool,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    if !helper.mor_data_files_dir_found {
        mor_get_data_files_dir(config_path_canonical, res, helper, cfg)
            .with_context(|| "Failed to find Morrowind's \"Data Files\" directory")?;
    }
    if let Some(raw_name) = line.split('=').nth(1) {
        let name = raw_name.trim();
        let path = helper.mor_data_files_dir.join(name);
        if path.exists() {
            let modification_time = path.metadata().map_or(None, |meta| meta.modified().ok());
            let path_str = path.to_string_lossy().into_owned();
            if prepend {
                if !res
                    .fallback_archives
                    .iter()
                    .any(|x| x.1 == path_str.to_lowercase())
                {
                    for &mut (ref mut id, _, _) in &mut res.fallback_archives {
                        *id = id
                            .checked_add(1)
                            .with_context(|| format!("Bug: overflow incrementing id = \"{id}\""))?;
                    }
                    res.fallback_archives
                        .insert(0, (0, path_str, modification_time));
                }
            } else {
                res.fallback_archives.push((
                    res.fallback_archives.len(),
                    path_str,
                    modification_time,
                ));
            }
        } else {
            let text = format!(
                "Archive \"{}\" not found at expected location \"{}\"",
                name,
                helper.mor_data_files_dir.join(name).display()
            );
            err_or_ignore(text, ignore_important_errors, false, cfg, log)?;
        }
    } else {
        let text = format!("Failed to parse line \"{line}\"");
        err_or_ignore(text, ignore_important_errors, false, cfg, log)?;
    }
    if !helper.mor_found {
        helper.mor_found = true;
    }
    Ok(())
}

fn omw_get_data_dir(
    line: &str,
    omw_data_dirs: &mut Vec<(usize, PathBuf)>,
    helper: &mut GetPluginsHelper,
    ignore_important_errors: bool,
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
        omw_data_dirs.push((helper.omw_data_counter, data));
        helper.omw_data_counter = helper.omw_data_counter.checked_add(1).with_context(|| {
            format!(
                "Bug: overflow incrementing helper.omw_data_counter = \"{}\"",
                helper.omw_data_counter
            )
        })?;
    } else {
        let text = format!("Failed to parse line \"{line}\"");
        err_or_ignore(text, ignore_important_errors, false, cfg, log)?;
    }
    if !helper.omw_found {
        helper.omw_found = true;
    }
    Ok(())
}

fn omw_get_plugin(
    name: &String,
    res: &mut LoadOrder,
    omw_all_plugins: &HashMap<String, PathBuf>,
    kind: &str,
    ignore_important_errors: bool,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    if let Some(path) = omw_all_plugins.get(name) {
        match kind {
            "plugin" => res.contents.push(path.to_string_lossy().into_owned()),
            "groundcover" => res.groundcovers.push(path.to_string_lossy().into_owned()),
            "fallback archive" => {
                let modification_time = path.metadata().map_or(None, |meta| meta.modified().ok());
                res.fallback_archives.push((
                    res.fallback_archives.len(),
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
        err_or_ignore(text, ignore_important_errors, false, cfg, log)?;
    }
    Ok(())
}

fn omw_get_cs_data_dir(
    omw_data_dirs: &mut Vec<(usize, PathBuf)>,
    helper: &mut GetPluginsHelper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let mut checked_paths: Vec<PathBuf> = Vec::new();
    macro_rules! check_omw_cs_data_path {
        ($omw_cs_data_path:expr) => {
            if $omw_cs_data_path.exists() {
                omw_data_dirs.push((helper.omw_data_counter, $omw_cs_data_path));
                helper.omw_data_counter =
                    helper.omw_data_counter.checked_add(1).with_context(|| {
                        format!(
                            "Bug: overflow incrementing helper.omw_data_counter = \"{}\"",
                            helper.omw_data_counter
                        )
                    })?;
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
