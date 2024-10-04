use crate::{
    err_or_ignore, err_or_ignore_thread_safe, msg, read_lines, Cfg, Helper, ListOptions, LoadOrder,
    Log,
};
use anyhow::{anyhow, Context, Result};
use dirs::{data_dir, document_dir};
use fs_err::read_dir;
use hashbrown::{hash_map::Entry, HashMap};
use paste::paste;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    env::current_dir,
    fmt::Write as _,
    path::{Path, PathBuf},
};
mod game_config;

pub fn scan(h: &mut Helper, cfg: &Cfg, log: &mut Log) -> Result<()> {
    if h.g.config_index == usize::MAX {
        game_config::get(h, cfg, log).with_context(|| "Failed to get game configuration file")?;
    }
    let config =
        h.t.game_configs
            .get_mut(h.g.config_index)
            .with_context(|| {
                format!(
                    "Bug: h.t.game_configs doesn't contain h.g.config_index = \"{}\"",
                    h.g.config_index
                )
            })?;
    if !config.load_order.scanned {
        let glb_h =
            GlobalGetPluginsHelper::new(&config.path, &config.path_canonical, &h.g.list_options);
        get_load_order(&mut config.load_order, &glb_h, cfg, log)
            .with_context(|| "Failed to get load order")?;
    }
    Ok(())
}

struct GlobalGetPluginsHelper<'a> {
    config_path: &'a Path,
    config_path_canonical: &'a Path,
    ignore: bool,
    force_base_dir: bool,
    base_dir_load_order: &'a PathBuf,
}

impl<'a> GlobalGetPluginsHelper<'a> {
    const fn new(
        config_path: &'a Path,
        config_path_canonical: &'a Path,
        list_options: &'a ListOptions,
    ) -> Self {
        Self {
            config_path,
            config_path_canonical,
            ignore: list_options.ignore_important_errors,
            force_base_dir: list_options.force_base_dir,
            base_dir_load_order: &list_options.indirect.base_dir_load_order,
        }
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Default)]
struct LocalGetPluginsHelper {
    mor_found: bool,
    omw_found: bool,
    omw_data_counter: usize,
    omw_data_line_counter: usize,
    mor_data_files_dir: PathBuf,
    mor_data_files_dir_found: bool,
    omw_all_plugins_found: bool,
}

macro_rules! increment_counter {
    ($($field:ident),+) => {
        $(paste!(
            fn [<increment_ $field>](&mut self) -> Result<()> {
                self.$field = self.$field.checked_add(1).with_context(|| {
                    format!(
                        "Bug: overflow incrementing {} = \"{}\"",
                        stringify!($field),
                        self.$field
                    )
                })?;
                Ok(())
            }
        );)+
    };
}

macro_rules! set_true_if_false {
    ($($field:ident),+) => {
        $(paste!(
            fn [<set_ $field>](&mut self) {
                if !self.$field {
                    self.$field = true;
                }
            }
        );)+
    };
}

impl LocalGetPluginsHelper {
    set_true_if_false!(mor_found, omw_found);
    increment_counter!(omw_data_counter, omw_data_line_counter);
}

#[allow(clippy::too_many_lines)]
fn get_load_order(
    load_order: &mut LoadOrder,
    glb_h: &GlobalGetPluginsHelper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let mut lcl_h = LocalGetPluginsHelper::default();
    let mut omw_data_dirs: Vec<(usize, PathBuf)> = Vec::new();
    let mut omw_plugins: Vec<String> = Vec::new();
    let mut omw_groundcovers: Vec<String> = Vec::new();
    let mut omw_fallback_archives: Vec<String> = Vec::new();
    let mut omw_all_plugins: HashMap<String, PathBuf> = HashMap::new();
    let mut text = format!(
        "Gathering plugins from game configuration file \"{}\"",
        glb_h.config_path.display()
    );
    msg(&text, 1, cfg, log)?;
    for line in read_lines(glb_h.config_path)
        .with_context(|| {
            format!(
                "Failed to read game configuration file \"{}\"",
                glb_h.config_path.display()
            )
        })?
        .map_while(Result::ok)
    {
        if !lcl_h.omw_found {
            if line.starts_with(&cfg.guts.mor_line_beginning_content) {
                if !lcl_h.mor_data_files_dir_found {
                    mor_get_data_dir(load_order, &mut lcl_h, glb_h, cfg)?;
                }
                mor_get_plugin(&line, load_order, &mut lcl_h, glb_h, cfg, log).with_context(
                    || format!("Failed to find Morrowind's plugin from line \"{line}\""),
                )?;
            } else if line.starts_with(&cfg.guts.mor_line_beginning_archive) {
                if !lcl_h.mor_data_files_dir_found {
                    mor_get_data_dir(load_order, &mut lcl_h, glb_h, cfg)?;
                }
                mor_get_archive(&line, load_order, false, &mut lcl_h, glb_h, cfg, log)
                    .with_context(|| {
                        format!("Failed to find Morrowind's archive from line \"{line}\"")
                    })?;
            } else { //
            }
        }
        if !lcl_h.mor_found {
            if line.starts_with(&cfg.guts.omw_line_beginning_data) {
                lcl_h.set_omw_found();
                if glb_h.force_base_dir {
                    lcl_h.increment_omw_data_line_counter()?;
                } else {
                    omw_get_data_dir(&line, &mut omw_data_dirs, &mut lcl_h, glb_h, cfg, log)
                        .with_context(|| "Failed to get OpenMW's data directory")?;
                }
            } else if line.starts_with(&cfg.guts.omw_line_beginning_fallback_archive) {
                omw_push_line_ending(&mut omw_fallback_archives, &line, &mut lcl_h);
            } else if line.starts_with(&cfg.guts.omw_line_beginning_groundcover) {
                omw_push_line_ending(&mut omw_groundcovers, &line, &mut lcl_h);
            } else if line.starts_with(&cfg.guts.omw_line_beginning_content) {
                omw_push_line_ending(&mut omw_plugins, &line, &mut lcl_h);
            } else { //
            }
        }
    }
    if lcl_h.omw_found {
        if !lcl_h.omw_all_plugins_found {
            if glb_h.force_base_dir {
                omw_data_dirs = vec![(0, glb_h.base_dir_load_order.clone())];
                omw_msg_force_base_dir(&mut text, &lcl_h, cfg, log)?;
            } else {
                if lcl_h.omw_data_line_counter == 0 {
                    omw_msg_no_data_lines(&mut text, cfg, log)?;
                }
                omw_get_cs_data_dir(&mut omw_data_dirs, &mut lcl_h, cfg, log)
                    .with_context(|| "Failed to find \"hidden\" OpenMW-CS data directory path")?;
                if omw_data_dirs.is_empty() {
                    omw_msg_no_data_dirs(&mut text, cfg, log)?;
                    let fallback_dir =
                        current_dir().with_context(|| "Failed to get current directory")?;
                    omw_data_dirs.push((0, fallback_dir));
                }
            }
            omw_all_plugins = get_all_plugins(&omw_data_dirs, &mut lcl_h, glb_h, cfg)
                .with_context(|| "Failed to find all OpenMW's plugins")?;
        };
        load_order.datas = omw_data_dirs;
        macro_rules! iter_kind_omw_get_plugin {
            ($kind:ident, $kind_str:expr) => {
                paste!(
                    [<omw_ $kind s>].iter().try_for_each(|$kind| -> Result<()> {
                        omw_get_plugin(
                            $kind,
                            load_order,
                            &omw_all_plugins,
                            $kind_str,
                            glb_h,
                            cfg,
                            log,
                        )
                        .with_context(|| format!("Failed to find OpenMW's {}", $kind_str))
                    })?;
                )
            }
        }
        iter_kind_omw_get_plugin!(plugin, "plugin");
        iter_kind_omw_get_plugin!(groundcover, "groundcover");
        iter_kind_omw_get_plugin!(fallback_archive, "fallback archive");
    } else if lcl_h.mor_found {
        if !lcl_h.mor_data_files_dir_found {
            mor_get_data_dir(load_order, &mut lcl_h, glb_h, cfg)?;
        }
        let missing_bsa = &cfg.guts.mor_line_missing_archive;
        mor_get_archive(missing_bsa, load_order, true, &mut lcl_h, glb_h, cfg, log)
            .with_context(|| "Failed to find Morrowind's base archive")?;
    } else { //
    }
    load_order.scanned = true;
    Ok(())
}

fn get_all_plugins(
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

fn mor_get_data_dir(
    load_order: &mut LoadOrder,
    lcl_h: &mut LocalGetPluginsHelper,
    glb_h: &GlobalGetPluginsHelper,
    cfg: &Cfg,
) -> Result<()> {
    if glb_h.force_base_dir {
        mor_get_base_dir(load_order, lcl_h, glb_h);
    } else {
        mor_get_data_files_dir(load_order, lcl_h, glb_h, cfg).with_context(|| {
            format!(
                "Failed to find Morrowind's \"{}\" directory",
                &cfg.guts.mor_data_files_dir,
            )
        })?;
    }
    lcl_h.mor_data_files_dir_found = true;
    Ok(())
}

fn mor_get_base_dir(
    load_order: &mut LoadOrder,
    lcl_h: &mut LocalGetPluginsHelper,
    glb_h: &GlobalGetPluginsHelper,
) {
    load_order
        .datas
        .push((0, glb_h.base_dir_load_order.clone()));
    lcl_h
        .mor_data_files_dir
        .clone_from(glb_h.base_dir_load_order);
}

fn mor_get_data_files_dir(
    load_order: &mut LoadOrder,
    lcl_h: &mut LocalGetPluginsHelper,
    glb_h: &GlobalGetPluginsHelper,
    cfg: &Cfg,
) -> Result<()> {
    match glb_h.config_path_canonical.parent() {
        Some(path) => {
            let data_files_dir = Path::new(path).join(&cfg.guts.mor_data_files_dir);
            if data_files_dir.exists() {
                load_order.datas.push((0, data_files_dir.clone()));
                lcl_h.mor_data_files_dir = data_files_dir;
                Ok(())
            } else {
                Err(anyhow!(
                    "Directory \"{}\" doesn't exist{}",
                    data_files_dir.display(),
                    custom_data_files_hint(cfg)
                ))
            }
        }
        None => {
            Err(anyhow!(
                "Failed to build Morrowind's \"{}\" directory path from game configuration \"{}\" file path{}",
                &cfg.guts.mor_data_files_dir,
                glb_h.config_path_canonical.display(),
                custom_data_files_hint(cfg)
            ))
        }
    }
}

fn custom_data_files_hint(cfg: &Cfg) -> String {
    format!("\nIt's expected for Morrowind's \"{}\" directory to be adjacent to the game configuration file\nConsider using --force-base-dir('-B') and --base-dir('-b') options to specify the directory: -Bb \"dir_path\"", cfg.guts.mor_data_files_dir)
}

fn mor_get_plugin(
    line: &str,
    load_order: &mut LoadOrder,
    lcl_h: &mut LocalGetPluginsHelper,
    glb_h: &GlobalGetPluginsHelper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    if let Some(raw_name) = line.split('=').nth(1) {
        let name = raw_name.trim();
        let path = lcl_h.mor_data_files_dir.join(name);
        if path.exists() {
            load_order
                .contents
                .push(path.to_string_lossy().into_owned());
        } else {
            let text = format!(
                "Plugin \"{name}\" not found at expected location \"{}\"",
                path.display()
            );
            err_or_ignore(text, glb_h.ignore, false, cfg, log)?;
        }
    } else {
        let text = format!("Failed to parse line \"{line}\"");
        err_or_ignore(text, glb_h.ignore, false, cfg, log)?;
    }
    lcl_h.set_mor_found();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn mor_get_archive(
    line: &str,
    load_order: &mut LoadOrder,
    prepend: bool,
    lcl_h: &mut LocalGetPluginsHelper,
    glb_h: &GlobalGetPluginsHelper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    if let Some(raw_name) = line.split('=').nth(1) {
        let name = raw_name.trim();
        let path = lcl_h.mor_data_files_dir.join(name);
        if path.exists() {
            let modification_time = path.metadata().map_or(None, |meta| meta.modified().ok());
            let path_str = path.to_string_lossy().into_owned();
            if prepend {
                if !load_order
                    .fallback_archives
                    .iter()
                    .any(|x| x.1 == path_str.to_lowercase())
                {
                    for &mut (ref mut id, _, _) in &mut load_order.fallback_archives {
                        *id = id
                            .checked_add(1)
                            .with_context(|| format!("Bug: overflow incrementing id = \"{id}\""))?;
                    }
                    load_order
                        .fallback_archives
                        .insert(0, (0, path_str, modification_time));
                }
            } else {
                load_order.fallback_archives.push((
                    load_order.fallback_archives.len(),
                    path_str,
                    modification_time,
                ));
            }
        } else {
            let text = format!(
                "Archive \"{name}\" not found at expected location \"{}\"",
                path.display()
            );
            err_or_ignore(text, glb_h.ignore, false, cfg, log)?;
        }
    } else {
        let text = format!("Failed to parse line \"{line}\"");
        err_or_ignore(text, glb_h.ignore, false, cfg, log)?;
    }
    lcl_h.set_mor_found();
    Ok(())
}

fn omw_get_data_dir(
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
        lcl_h.increment_omw_data_counter()?;
    } else {
        let text = format!("Failed to parse line \"{line}\"");
        err_or_ignore(text, glb_h.ignore, false, cfg, log)?;
    }
    Ok(())
}

fn omw_push_line_ending(vec: &mut Vec<String>, line: &str, lcl_h: &mut LocalGetPluginsHelper) {
    lcl_h.set_omw_found();
    if let Some(raw_name) = line.split('=').nth(1) {
        vec.push(raw_name.trim().to_owned());
    }
}

fn omw_get_plugin(
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

fn omw_get_cs_data_dir(
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
                lcl_h.increment_omw_data_counter()?;
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

fn omw_msg_force_base_dir(
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

fn omw_msg_no_data_lines(text: &mut String, cfg: &Cfg, log: &mut Log) -> Result<()> {
    text.clear();
    write!(
        text,
        "Warning: game configuration file doesn't contain \"{}\" lines",
        &cfg.guts.omw_line_beginning_data,
    )?;
    msg(&text, 0, cfg, log)
}

fn omw_msg_no_data_dirs(text: &mut String, cfg: &Cfg, log: &mut Log) -> Result<()> {
    text.clear();
    write!(
        text,
        "Failed to get any \"{}\" directory: falling back to current directory to look for plugins",
        &cfg.guts.omw_line_beginning_data,
    )?;
    msg(&text, 0, cfg, log)
}
