use crate::{msg, msg_no_log, references_sorted, Cfg, Helper, Log, Mode};
use anyhow::{Context, Result};
use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
};
use tes3::esp::{Plugin, Reference, TES3Object};

pub(crate) fn write_output_plugin(
    name: &str,
    plugin: &mut Plugin,
    old_plugin: &mut Plugin,
    h: &Helper,
    // contains_non_external_refs: bool,
    // dry_run: bool,
    // mode: &Mode,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let name_path = PathBuf::from(name);
    if name_path.exists() {
        if !cfg.no_compare {
            old_plugin
                .load_path(name)
                .with_context(|| format!("Failed to read previous output plugin \"{}\"", name))?;
            if is_plugin_equal_to_previous(plugin, old_plugin) {
                let mut text = format!("Output plugin \"{}\" is equal to previous version", name);
                if cfg.verbose < 1 {
                    msg_no_log(&text, 0, cfg);
                }
                text = format!("{}{}", text, cfg.guts.prefix_list_stats);
                msg(text, 1, cfg, log)?;
                return Ok(());
            };
        }
    } else if let Some(out_dir) = name_path.parent() {
        if out_dir != Path::new("") && !out_dir.exists() {
            if !h.g.dry_run {
                create_dir_all(out_dir)
                    .with_context(|| format!("Failed to create output plugin directory \"{}\"", out_dir.display()))?;
            }

            let text = if !h.g.dry_run {
                format!("Output plugin directory \"{}\" was created", out_dir.display())
            } else {
                format!("Output plugin directory \"{}\" would be created", out_dir.display())
            };
            msg(text, 0, cfg, log)?;
        }
    };
    if !h.g.dry_run {
        plugin
            .save_path(name)
            .with_context(|| format!("Failed to write output plugin to \"{}\"", name))?;
    }
    let mut text = if !h.g.dry_run {
        format!("Output plugin \"{}\" was written", name)
    } else {
        format!("Output plugin \"{}\" would be written", name)
    };
    if h.g.contains_non_external_refs {
        match h.g.mode {
            Mode::Grass => {}
            _ => {
                if !h.g.dry_run {
                    text.push_str(". It contains reindexed references most likely, so new game is recommended.");
                } else {
                    text.push_str(". It would contain reindexed references most likely, so new game would be recommended.");
                }
            }
        }
    }
    if cfg.verbose < 1 {
        msg_no_log(&text, 0, cfg);
    }
    text = format!("{}{}", text, cfg.guts.prefix_list_stats);
    msg(text, 1, cfg, log)?;
    Ok(())
}

fn is_plugin_equal_to_previous(new_plugin: &Plugin, old_plugin: &Plugin) -> bool {
    for (new, old) in new_plugin.objects.iter().zip(old_plugin.objects.iter()) {
        match new {
            TES3Object::Cell(new_cell) => match old {
                TES3Object::Cell(old_cell) => {
                    if new_cell.flags != old_cell.flags
                        || new_cell.name != old_cell.name
                        || new_cell.data != old_cell.data
                        || new_cell.region != old_cell.region
                        || new_cell.map_color != old_cell.map_color
                        || new_cell.water_height != old_cell.water_height
                        || new_cell.atmosphere_data != old_cell.atmosphere_data
                    {
                        return false;
                    };
                    let mut new_refs: Vec<&Reference> = new_cell.references.values().collect();
                    references_sorted(&mut new_refs);
                    let mut old_refs: Vec<&Reference> = old_cell.references.values().collect();
                    references_sorted(&mut old_refs);
                    if new_refs != old_refs {
                        return false;
                    }
                }
                _ => {
                    return false;
                }
            },
            TES3Object::Creature(new_creature) => match old {
                TES3Object::Creature(old_creature) => {
                    let mut new_creature = new_creature.clone();
                    if new_creature.scale == Some(1.0) {
                        new_creature.scale = None;
                    }
                    if &new_creature != old_creature {
                        return false;
                    }
                }
                _ => {
                    return false;
                }
            },
            _ => {
                if new != old {
                    return false;
                }
            }
        }
    }
    true
}
