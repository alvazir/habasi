use super::Log;
use crate::{make_turn_normal_grass, write_output_plugin, Cfg, Helper, Out, Plugin};
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};

pub fn process_turn_normal_grass(
    name: &str,
    out: &mut Out,
    old_plugin: &mut Plugin,
    h: &mut Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    if h.g.list_options.turn_normal_grass {
        let (
            plugin_deleted_content_name,
            mut plugin_deleted_content,
            plugin_grass_name,
            mut plugin_grass,
        ) = make_turn_normal_grass(name, out, h, cfg, log)?;
        if !h.g.list_options.exclude_deleted_records {
            write_output_plugin(
                &plugin_deleted_content_name,
                &mut plugin_deleted_content,
                old_plugin,
                2,
                h,
                cfg,
                log,
            )
            .with_context(|| {
                format!("Failed to write output plugin {plugin_deleted_content_name:?}")
            })?;
        }
        write_output_plugin(
            &plugin_grass_name,
            &mut plugin_grass,
            old_plugin,
            2,
            h,
            cfg,
            log,
        )
        .with_context(|| format!("Failed to write output plugin {plugin_grass_name:?}"))?;
    }
    Ok(())
}

pub fn get_tng_dir_and_plugin_names(name: &str, cfg: &Cfg) -> Result<(PathBuf, String, String)> {
    let name_path = PathBuf::from(name);
    let name_stem = match name_path.file_stem() {
        None => {
            return Err(anyhow!(
            "Failed to find output plugin file name without path and extension from input \"{}\"",
            name
        ))
        }
        Some(stem) => stem.to_string_lossy().into_owned(),
    };
    let dir = name_path
        .parent()
        .map_or_else(PathBuf::new, Path::to_path_buf);
    let plugin_deleted_content_name_pathbuf = dir.join(format!(
        "{}{}",
        name_stem, &cfg.guts.turn_normal_grass_plugin_name_suffix_content
    ));
    let plugin_deleted_content_name = plugin_deleted_content_name_pathbuf
        .to_string_lossy()
        .into_owned();
    let plugin_grass_name_pathbuf = dir.join(format!(
        "{}{}",
        name_stem, &cfg.guts.turn_normal_grass_plugin_name_suffix_groundcover
    ));
    let plugin_grass_name = plugin_grass_name_pathbuf.to_string_lossy().into_owned();
    Ok((dir, plugin_deleted_content_name, plugin_grass_name))
}

pub fn get_tng_content_name_low(name: &str, h: &Helper, cfg: &Cfg) -> Result<String> {
    if !h.g.list_options.turn_normal_grass && !h.g.list_options.use_load_order {
        Ok(String::new())
    } else {
        let (_, tng_content_name, _) = get_tng_dir_and_plugin_names(name, cfg)
            .with_context(|| "Failed to get turn normal grass directory or plugin names")?;
        Ok(tng_content_name.to_lowercase())
    }
}
