use super::{GlobalGetPluginsHelper, LocalGetPluginsHelper};
use crate::{err_or_ignore, increment, Cfg, LoadOrder, Log};
use anyhow::{anyhow, Context, Result};
use std::path::Path;

pub(super) fn get_data_dir(
    load_order: &mut LoadOrder,
    lcl_h: &mut LocalGetPluginsHelper,
    glb_h: &GlobalGetPluginsHelper,
    cfg: &Cfg,
) -> Result<()> {
    if glb_h.force_base_dir {
        get_base_dir(load_order, lcl_h, glb_h);
    } else {
        get_data_files_dir(load_order, lcl_h, glb_h, cfg).with_context(|| {
            format!(
                "Failed to find Morrowind's \"{}\" directory",
                &cfg.guts.mor_data_files_dir,
            )
        })?;
    }
    lcl_h.mor_data_files_dir_found = true;
    Ok(())
}

pub(super) fn get_plugin(
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
pub(super) fn get_archive(
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
                        *id = increment!(id);
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

fn get_base_dir(
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

fn get_data_files_dir(
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
