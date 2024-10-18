use crate::{increment, msg, read_lines, Cfg, Helper, ListOptions, LoadOrder, Log};
use anyhow::{Context, Result};
use hashbrown::HashMap;
use paste::paste;
use std::{
    env::current_dir,
    path::{Path, PathBuf},
};
mod game_config;
mod mor;
mod omw;

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
                    mor::get_data_dir(load_order, &mut lcl_h, glb_h, cfg)?;
                }
                mor::get_plugin(&line, load_order, &mut lcl_h, glb_h, cfg, log).with_context(
                    || format!("Failed to find Morrowind's plugin from line \"{line}\""),
                )?;
            } else if line.starts_with(&cfg.guts.mor_line_beginning_archive) {
                if !lcl_h.mor_data_files_dir_found {
                    mor::get_data_dir(load_order, &mut lcl_h, glb_h, cfg)?;
                }
                mor::get_archive(&line, load_order, false, &mut lcl_h, glb_h, cfg, log)
                    .with_context(|| {
                        format!("Failed to find Morrowind's archive from line \"{line}\"")
                    })?;
            } else { //
            }
        }
        if !lcl_h.mor_found {
            if line.starts_with(&cfg.guts.omw_line_beginning_data) {
                lcl_h.set_omw_found();
                lcl_h.omw_data_line_counter = increment!(lcl_h.omw_data_line_counter);
                if !glb_h.force_base_dir {
                    omw::get_data_dir(&line, &mut omw_data_dirs, &mut lcl_h, glb_h, cfg, log)
                        .with_context(|| "Failed to get OpenMW's data directory")?;
                }
            } else if line.starts_with(&cfg.guts.omw_line_beginning_fallback_archive) {
                omw::push_line_ending(&mut omw_fallback_archives, &line, &mut lcl_h);
            } else if line.starts_with(&cfg.guts.omw_line_beginning_groundcover) {
                omw::push_line_ending(&mut omw_groundcovers, &line, &mut lcl_h);
            } else if line.starts_with(&cfg.guts.omw_line_beginning_content) {
                omw::push_line_ending(&mut omw_plugins, &line, &mut lcl_h);
            } else { //
            }
        }
    }
    if lcl_h.omw_found {
        if !lcl_h.omw_all_plugins_found {
            if glb_h.force_base_dir {
                omw_data_dirs = vec![(0, glb_h.base_dir_load_order.clone())];
                omw::msg_force_base_dir(&mut text, &lcl_h, cfg, log)?;
            } else {
                if lcl_h.omw_data_line_counter == 0 {
                    omw::msg_no_data_lines(&mut text, cfg, log)?;
                }
                omw::get_cs_data_dir(&mut omw_data_dirs, &mut lcl_h, cfg, log)
                    .with_context(|| "Failed to find \"hidden\" OpenMW-CS data directory path")?;
                if omw_data_dirs.is_empty() {
                    omw::msg_no_data_dirs(&mut text, cfg, log)?;
                    let fallback_dir =
                        current_dir().with_context(|| "Failed to get current directory")?;
                    omw_data_dirs.push((0, fallback_dir));
                }
            }
            omw_all_plugins = omw::get_all_plugins(&omw_data_dirs, &mut lcl_h, glb_h, cfg)
                .with_context(|| "Failed to find all OpenMW's plugins")?;
        };
        load_order.datas = omw_data_dirs;
        macro_rules! iter_kind_omw_get_plugin {
            ($kind:ident, $kind_str:expr) => {
                paste!(
                    [<omw_ $kind s>].iter().try_for_each(|$kind| -> Result<()> {
                        omw::get_plugin(
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
            mor::get_data_dir(load_order, &mut lcl_h, glb_h, cfg)?;
        }
        let missing_bsa = &cfg.guts.mor_line_missing_archive;
        mor::get_archive(missing_bsa, load_order, true, &mut lcl_h, glb_h, cfg, log)
            .with_context(|| "Failed to find Morrowind's base archive")?;
    } else { //
    }
    load_order.scanned = true;
    Ok(())
}
