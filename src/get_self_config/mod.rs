use crate::{create_dir_early, get_base_dir, structs::Mode};
use anyhow::{Context, Result};
use std::{fs, path::PathBuf};
mod options;
mod settings;
mod util;
use confique::toml::{template, FormatOptions};
use options::{get_options, Options};
use settings::{get_settings, Settings};
use util::{check_mode, get_exe_name_and_dir, get_lists, get_log_file, get_settings_file};

pub(crate) struct Cfg {
    pub(crate) merge: Vec<Vec<String>>,
    pub(crate) no_compare: bool,
    pub(crate) grass: bool,
    pub(crate) log: Option<PathBuf>,
    pub(crate) no_log: bool,
    pub(crate) settings: PathBuf,
    pub(crate) settings_write: bool,
    pub(crate) dry_run: bool,
    pub(crate) mode: Mode,
    pub(crate) base_dir: PathBuf,
    pub(crate) no_ignore_errors: bool,
    pub(crate) strip_masters: bool,
    pub(crate) reindex: bool,
    pub(crate) debug: bool,
    pub(crate) verbose: u8,
    pub(crate) quiet: bool,
    pub(crate) show_all_missing_refs: bool,
    pub(crate) guts: Guts,
}

pub(crate) struct Guts {
    pub(crate) grass_filter: Vec<String>,
    pub(crate) header_version: f32,
    pub(crate) header_author: String,
    pub(crate) header_description: String,
    pub(crate) prefix_combined_stats: String,
    pub(crate) prefix_list_stats: String,
}

impl Cfg {
    fn new(opt: Options, set: Settings, settings_file: PathBuf, exe: Option<String>, dir: Option<PathBuf>) -> Result<Cfg> {
        macro_rules! opt_or_set_bool {
            ($name:ident) => {
                match opt.$name {
                    true => opt.$name,
                    false => set.options.$name,
                }
            };
        }
        macro_rules! opt_or_set_some {
            ($name:ident) => {
                match opt.$name {
                    Some(value) => value,
                    None => set.options.$name,
                }
            };
        }
        let no_log = opt_or_set_bool!(no_log);
        let mode = opt_or_set_some!(mode);
        let base_dir_string = opt_or_set_some!(base_dir);
        Ok(Cfg {
            merge: get_lists(opt.merge, set.options.merge),
            no_compare: opt_or_set_bool!(no_compare),
            grass: opt_or_set_bool!(grass),
            no_log,
            log: get_log_file(no_log, opt_or_set_some!(log), exe, dir)?,
            settings: settings_file,
            settings_write: opt.settings_write,
            dry_run: opt_or_set_bool!(dry_run),
            mode: check_mode(&mode)?,
            base_dir: get_base_dir(&base_dir_string).with_context(|| "Failed to get default base_dir")?,
            no_ignore_errors: opt_or_set_bool!(no_ignore_errors),
            strip_masters: opt_or_set_bool!(strip_masters),
            reindex: opt_or_set_bool!(reindex),
            debug: opt_or_set_bool!(debug),
            verbose: if opt.verbose == 0 { set.options.verbose } else { opt.verbose },
            quiet: opt_or_set_bool!(quiet),
            show_all_missing_refs: opt_or_set_bool!(show_all_missing_refs),
            guts: Guts {
                grass_filter: set.guts.grass_filter,
                header_version: set.guts.header_version,
                header_author: set.guts.header_author,
                header_description: set.guts.header_description,
                prefix_combined_stats: set.guts.prefix_combined_stats,
                prefix_list_stats: set.guts.prefix_list_stats,
            },
        })
    }
}

pub(crate) fn get_self_config() -> Result<Cfg> {
    let options = get_options()?;
    let (exe, dir) = get_exe_name_and_dir();
    let settings_file =
        get_settings_file(&exe, &dir, &options.settings).with_context(|| "Failed to get program settings file path")?;
    let settings = get_settings(&settings_file).with_context(|| "Failed to get default or provided settings")?;
    if options.settings_write {
        let toml = template::<Settings>(FormatOptions::default());
        create_dir_early(&settings_file, "settings")?;
        fs::write(&settings_file, toml)
            .with_context(|| format!("Failed to write default program settings into \"{}\"", settings_file.display()))?;
    }
    let configuration = Cfg::new(options, settings, settings_file, exe, dir).with_context(|| "Failed to configure program")?;
    Ok(configuration)
}
