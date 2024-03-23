/*
 *  Habasi - TES3 plugin merging and utility tool
 *
 *  Copyright (C) 2023 alvazir
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use anyhow::{Context, Result};
use std::{
    io::{Error as IOError, ErrorKind},
    process::exit,
    time::Instant,
};
use tes3::esp::Plugin;
mod assets;
mod config;
mod input;
mod load_order;
mod output;
mod stats;
mod structs;
mod util;
use assets::{bsa::Bsa, make_tng_meshes::make_tng_meshes};
use config::{get_self_config, Cfg};
use input::process_records;
use load_order::{get_game_config::get_game_config, get_load_order::get_load_order};
use output::{
    make_output_plugin::make_output_plugin, make_turn_normal_grass::make_turn_normal_grass, transform_output::transform,
    write_output_plugin::write_output_plugin,
};
// use peak_alloc::PeakAlloc; // slows down the program too much
use stats::{Stats, StatsUpdateKind};
use structs::{
    Assets, CellExtGrid, CellMeta, Dial, DialMeta, FallbackStatics, FileInBsa, GlobalMaster, GlobalVtexId, HeaderText, Helper,
    IgnoredRefError, ListOptions, LoadOrder, LocalMaster, LocalMergedMaster, LocalVtexId, MastId, MasterNameLow, MergedPluginMeta,
    MergedPluginRefr, Mode, MovedInstanceGrids, MovedInstanceId, OldRefSources, Out, PluginInfo, PluginName, RefSources, RefrId,
    RegexPluginInfo, TurnNormalGrass,
};
use util::{
    check_presets, create_dir_early, err_or_ignore, err_or_ignore_thread_safe, get_append_to_use_load_order_string, get_base_dir_path,
    get_cell_name, get_expanded_plugin_list, get_game_config_string, get_regex_plugin_list, get_skip_from_use_load_order_string,
    get_skip_plugin_name_low, get_tng_content_name_low, get_tng_dir_and_plugin_names, msg, msg_no_log, process_moved_instances,
    process_plugin, process_turn_normal_grass, read_lines, references_sorted, scan_load_order, select_header_description,
    should_skip_list, show_global_list_options, show_ignored_ref_errors, show_log_path, show_removed_record_ids,
    show_settings_version_message, show_settings_written, truncate_header_text, Log, CRC64, SNDG_ID_MAX_LEN, SNDG_ID_SUFFIX_LEN,
    SNDG_MAX_SOUND_FLAG,
};

// #[global_allocator]
// static PEAK_ALLOC: PeakAlloc = PeakAlloc; // slows down the program too much

fn main() {
    #[allow(clippy::use_debug, clippy::print_stderr)]
    match run() {
        Ok(()) => {
            // println!("PEAK MEMORY USAGE: {:.0}MB", PEAK_ALLOC.peak_usage_as_mb()); // slows down the program too much
            exit(0)
        }
        Err(error) => {
            eprintln!("{error:?}");
            exit(1);
        }
    }
}

fn run() -> Result<()> {
    let timer_total = Instant::now();
    let cfg = get_self_config()?;
    let mut log = Log::new(&cfg)?;
    show_log_path(&cfg, &mut log)?;
    if cfg.settings_file.write {
        show_settings_written(&cfg, &mut log)?;
        return Ok(());
    }
    show_settings_version_message(&cfg, &mut log)?;
    let mut h = Helper::new();
    show_global_list_options(&cfg, &mut log)?;
    let merge_override = check_presets(&mut h, &cfg, &mut log)?;
    let merge = if merge_override.is_empty() { &cfg.merge } else { &merge_override };
    if merge.is_empty() {
        let text = "Nothing to proceed: at least one --merge or --preset-* option is required";
        msg(text, 0, &cfg, &mut log)?;
        return Ok(());
    }
    let mut output_plugin = Plugin::new();
    let mut old_output_plugin = Plugin::new();
    for list in merge {
        process_list(list, &mut output_plugin, &mut old_output_plugin, &mut h, &cfg, &mut log)
            .with_context(|| format!("Failed to process list \"{}\"", list.join(", ")))?;
    }
    h.total_commit(timer_total, &cfg, &mut log)?;
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn process_list(
    list: &[String],
    output_plugin: &mut Plugin,
    old_output_plugin: &mut Plugin,
    h: &mut Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let timer_global = Instant::now();
    let mut out = Out::default();
    let name = if list.is_empty() {
        msg("Skipping empty list", 0, cfg, log)?;
        return Ok(());
    } else {
        list.first().with_context(|| "Bug: failed to get name from list")?
    };
    let (index, list_options) = cfg.list_options.get_list_options(list, cfg, log)?;
    let expanded_plugin_list = get_expanded_plugin_list(list, index, &list_options, h, cfg, log)
        .with_context(|| "Failed to expand plugin list by scanning load order")?;
    #[allow(clippy::shadow_same)]
    let mut list = list;
    if !expanded_plugin_list.is_empty() {
        list = expanded_plugin_list
            .get(..)
            .with_context(|| "Bug: indexing slicing expanded_plugin_list[..]")?;
    };
    let regex_plugin_list = get_regex_plugin_list(list, index, &list_options, cfg, log)
        .with_context(|| "Failed to expand plugin list with regex/glob patterns")?;
    if !regex_plugin_list.is_empty() {
        list = regex_plugin_list
            .get(..)
            .with_context(|| "Bug: indexing slicing regex_plugin_list[..]")?;
    };
    if should_skip_list(name, list, index, &list_options, cfg, log)? {
        return Ok(());
    };
    let plugin_list = list
        .get(index..)
        .with_context(|| format!("Bug: indexing slicing list[{index}..]"))?;
    let mut text: String;
    if cfg.show_plugins {
        text = format!(
            "List \"{}\" contains {} files(list is ready to be copied into settings file):\n\"{}\"",
            &name,
            plugin_list.len(),
            plugin_list.join("\",\n\"")
        );
        msg(&text, 0, cfg, log)?;
    }
    text = format!("Processing list \"{}\" with options: {}", &name, list_options.show()?);
    msg(&text, 1, cfg, log)?;
    h.global_init(list_options);
    let tng_content_name_low = get_tng_content_name_low(name, h, cfg)?;
    let skip_plugin_name_low = get_skip_plugin_name_low(h);
    for plugin_name in plugin_list {
        let plugin_name_low = plugin_name.to_lowercase();
        if cfg
            .guts
            .plugin_extensions_to_ignore
            .iter()
            .any(|ext| plugin_name_low.ends_with(ext))
        {
            text = format!("  Skipped processing plugin \"{plugin_name}\" because it has extension to ignore");
            msg(&text, cfg.guts.skipped_processing_plugins_msg_verbosity, cfg, log)?;
            h.total_add_skipped_processing_plugin(text);
            continue;
        }
        if !tng_content_name_low.is_empty() && plugin_name_low.ends_with(&tng_content_name_low) {
            text = format!("  Skipped processing plugin \"{plugin_name}\" trying to recreate it from scratch");
            msg(&text, cfg.guts.skipped_processing_plugins_msg_verbosity, cfg, log)?;
            h.total_add_skipped_processing_plugin(text);
        } else if !skip_plugin_name_low.is_empty() && plugin_name_low.ends_with(&skip_plugin_name_low) {
            text = format!("  Skipped processing plugin \"{plugin_name}\" due to \"skip_from_use_load_order\"");
            msg(&text, cfg.guts.skipped_processing_plugins_msg_verbosity, cfg, log)?;
            h.total_add_skipped_processing_plugin(text);
        } else {
            if let Err(err) = process_plugin(plugin_name, &mut out, name, h, cfg, log) {
                {
                    if let Some(inner) = err.downcast_ref::<IOError>() {
                        if matches!(inner.kind(), ErrorKind::InvalidData) {
                            if let Some(tag) = inner.to_string().strip_prefix("Unexpected Tag: ") {
                                if cfg.guts.unexpected_tags_to_ignore.contains(&tag.to_lowercase()) {
                                    text = format!(
                                        "  Skipped processing plugin \"{plugin_name}\" because it contains unexpected record type to ignore: \"{tag}\""
                                    );
                                    msg(&text, cfg.guts.skipped_processing_plugins_msg_verbosity, cfg, log)?;
                                    h.total_add_skipped_processing_plugin(text);
                                } else {
                                    err_or_ignore(format!("{err:#}"), h.g.list_options.ignore_important_errors, true, cfg, log)
                                        .with_context(|| "Failed to process plugin")?;
                                }
                                continue;
                            }
                        };
                    }
                    err_or_ignore(format!("{err:#}"), h.g.list_options.ignore_important_errors, false, cfg, log)
                        .with_context(|| "Failed to process plugin")?;
                    continue;
                };
            };
            h.local_commit(cfg, log)?;
        }
    }
    if h.g.stats.all_plugins_ignored() {
        msg("Skipping list because all plugins were skipped", 0, cfg, log)?;
        return Ok(());
    }
    process_moved_instances(&mut out, h)?;
    out = transform(name, out, h, cfg, log)?;
    process_turn_normal_grass(name, &mut out, old_output_plugin, h, cfg, log)?;
    make_output_plugin(name, out, output_plugin, h, cfg, log).with_context(|| format!("Failed to make output plugin {name:?}"))?;
    write_output_plugin(name, output_plugin, old_output_plugin, 1, h, cfg, log)
        .with_context(|| format!("Failed to write output plugin {name:?}"))?;
    h.global_commit(timer_global, output_plugin, cfg, log)?;
    Ok(())
}
