use super::Log;
use crate::{get_tng_dir_and_plugin_names, load_order::scan, Cfg, Helper};
use anyhow::{Context as _, Result};

#[allow(clippy::module_name_repetitions)]
pub fn check_presets(h: &mut Helper, cfg: &Cfg, log: &mut Log) -> Result<Vec<Vec<String>>> {
    let mut merge_override: Vec<Vec<String>> = Vec::new();
    if cfg.presets.present {
        h.g.list_options = cfg.list_options.get_pristine();
        if cfg.presets.check_references {
            merge_override = vec![cfg.guts.preset_config_check_references.clone()];
        };
        if cfg.presets.turn_normal_grass {
            let mut preset_config_turn_normal_grass =
                cfg.guts.preset_config_turn_normal_grass.clone();
            if cfg.presets.check_references {
                preset_config_turn_normal_grass.extend(
                    cfg.guts
                        .preset_config_turn_normal_grass_add_with_check_references
                        .clone(),
                );
            }
            merge_override = vec![preset_config_turn_normal_grass];
        };
        if cfg.presets.merge_load_order {
            let mut preset_config_merge_load_order =
                cfg.guts.preset_config_merge_load_order.clone();
            if cfg.presets.check_references {
                preset_config_merge_load_order.extend(
                    cfg.guts
                        .preset_config_merge_load_order_add_with_check_references
                        .clone(),
                );
            }
            if cfg.presets.turn_normal_grass {
                preset_config_merge_load_order.extend(
                    cfg.guts
                        .preset_config_merge_load_order_add_with_turn_normal_grass
                        .clone(),
                );
            }
            // COMMENT: process options like base_dir earlier than expected for the scan to work
            h.g.list_options = cfg
                .list_options
                .get_mutated(&preset_config_merge_load_order, cfg, log)
                .with_context(|| "Failed to get list options")?
                .1;
            scan(h, cfg, log).with_context(|| "Failed to scan load order")?;
            merge_override = vec![preset_config_merge_load_order];
            let groundcovers_len =
                h.t.game_configs
                    .get(h.g.config_index)
                    .with_context(|| {
                        format!(
                            "Bug: h.t.game_configs doesn't contain h.g.config_index = \"{}\"",
                            h.g.config_index
                        )
                    })?
                    .load_order
                    .groundcovers
                    .len();
            if groundcovers_len > 0 {
                let mut preset_config_merge_load_order_grass =
                    cfg.guts.preset_config_merge_load_order_grass.clone();
                if cfg.presets.turn_normal_grass {
                    let (_, _, plugin_grass_name) = get_tng_dir_and_plugin_names(
                        cfg.guts
                            .preset_config_merge_load_order
                            .first()
                            .with_context(|| {
                                "Bug: cfg.guts.preset_config_merge_load_order is empty"
                            })?,
                        cfg,
                    )
                    .with_context(|| "Failed to get turn normal grass directory or plugin names")?;
                    preset_config_merge_load_order_grass.push(format!(
                        "{}{}",
                        cfg.guts.list_options_prefix_append_to_use_load_order, plugin_grass_name
                    ));
                    merge_override.push(preset_config_merge_load_order_grass);
                } else if groundcovers_len > 1 {
                    merge_override.push(preset_config_merge_load_order_grass);
                } else { //
                }
            }
        };
    }
    Ok(merge_override)
}
