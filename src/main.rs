use anyhow::{Context, Result};
use std::{process::exit, time::Instant};
use tes3::esp::Plugin;
mod get_self_config;
mod make_output_plugin;
mod process_records;
mod stats;
mod structs;
mod util;
mod write_output_plugin;
use get_self_config::{get_self_config, Cfg};
use make_output_plugin::make_output_plugin;
use process_records::process_records;
use stats::{Stats, StatsUpdateKind};
use structs::{
    CellExtGrid, CellMeta, Dial, DialMeta, GlobalMaster, GlobalVtexId, Helper, IgnoredRefError, LocalMaster, LocalMergedMaster,
    LocalVtexId, MasterNameLow, MergedPluginMeta, MergedPluginRefr, Mode, MovedInstanceGrids, MovedInstanceId, Out, PluginNameLow,
    RefrId,
};
use util::{
    create_dir_early, get_base_dir, get_list_parameters, msg, msg_no_log, process_moved_instances, references_sorted,
    show_ignored_ref_errors, show_list_options, show_log_path, show_settings_written, Log,
};
use write_output_plugin::write_output_plugin;

fn main() {
    match run() {
        Ok(()) => exit(0),
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
    if cfg.settings_write {
        show_settings_written(&cfg, &mut log)?;
        return Ok(());
    }
    let text = format!(
        "Global list options: {}",
        show_list_options(
            cfg.dry_run,
            &cfg.mode,
            &cfg.base_dir,
            cfg.no_ignore_errors,
            cfg.strip_masters,
            cfg.reindex,
            cfg.debug
        )
    );
    msg(text, 1, &cfg, &mut log)?;
    if cfg.merge.is_empty() {
        msg("Nothing to proceed", 0, &cfg, &mut log)?;
        return Ok(());
    }
    let mut h = Helper::default();
    let mut output_plugin = Plugin::new();
    let mut old_output_plugin = Plugin::new();
    for list in &cfg.merge {
        process_list(&list[..], &mut output_plugin, &mut old_output_plugin, &mut h, &cfg, &mut log)
            .with_context(|| format!("Failed to process list \"{}\"", list.join(", ")))?;
    }
    h.total_commit(timer_total, &cfg, &mut log)?;
    Ok(())
}

fn process_list(
    plugin_list: &[String],
    output_plugin: &mut Plugin,
    old_output_plugin: &mut Plugin,
    h: &mut Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let timer_global = Instant::now();
    let mut out = Out::default();
    let name = if !plugin_list.is_empty() {
        &plugin_list[0]
    } else {
        msg("Skipping empty list", 0, cfg, log)?;
        return Ok(());
    };
    // TODO move this up to global_init into global_init?
    let (index, dry_run, mode, base_dir, no_ignore_errors, strip_masters, reindex, debug) = get_list_parameters(plugin_list, cfg)?;
    if plugin_list.len() < (index + 1) {
        let text = format!("Output plugin \"{}\" processing skipped due to empty list", name);
        msg(text, 0, cfg, log)?;
        return Ok(());
    }
    if !cfg.grass {
        if let Mode::Grass = mode {
            let text = format!("Output plugin \"{}\" processing skipped due to \"grass=false\"", name);
            msg(text, 0, cfg, log)?;
            return Ok(());
        };
    }
    let text = format!(
        "Processing list \"{}\" with options: {}",
        &name,
        show_list_options(dry_run, &mode, &base_dir, no_ignore_errors, strip_masters, reindex, debug)
    );
    msg(text, 1, cfg, log)?;
    h.global_init(dry_run, mode, base_dir, no_ignore_errors, strip_masters, reindex, debug);
    for plugin_name in &plugin_list[index..] {
        let mut plugin_pathbuf = h.g.base_dir.clone();
        plugin_pathbuf.push(plugin_name);
        let plugin_path = plugin_pathbuf.to_string_lossy().into_owned();
        let text = format!("  Processing plugin \"{}\"", &plugin_path);
        msg(text, 2, cfg, log)?;
        h.local_init(&plugin_path)?;
        let mut plugin: Plugin = Plugin::new();
        plugin
            .load_path(&plugin_path)
            .with_context(|| format!("Failed to read plugin \"{}\"", &plugin_path))?;
        process_records(plugin, &mut out, name, h, cfg, log)
            .with_context(|| format!("Failed to process records from plugin \"{}\"", &plugin_path))?;
        h.local_commit(cfg, log)?;
    }
    process_moved_instances(&mut out, h)?;
    make_output_plugin(name, out, output_plugin, h, cfg, log).with_context(|| format!("Failed to make output plugin \"{}\"", name))?;
    write_output_plugin(
        name,
        output_plugin,
        old_output_plugin,
        h,
        // h.g.contains_non_external_refs,
        // dry_run,
        // &mode,
        cfg,
        log,
    )
    .with_context(|| format!("Failed to write output plugin \"{}\"", name))?;
    h.global_commit(timer_global, output_plugin, old_output_plugin, cfg, log)?;
    Ok(())
}
