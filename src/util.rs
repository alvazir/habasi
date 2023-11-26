use crate::{
    get_game_config, get_load_order, make_turn_normal_grass, process_records, write_output_plugin, Cfg, Helper, IgnoredRefError,
    ListOptions, Mode, Out, Plugin, PluginName,
};
use anyhow::{anyhow, Context, Result};
use crc::{Crc, CRC_64_ECMA_182};
use fs_err::{copy, create_dir_all, File};
use std::{
    io::{self, BufRead, BufWriter, Write},
    path::{Path, PathBuf},
};
use tes3::esp::{Cell, CellFlags, Reference};

pub(crate) const CRC64: Crc<u64> = Crc::<u64>::new(&CRC_64_ECMA_182);
pub(crate) const SNDG_ID_MAX_LEN: usize = 32;
pub(crate) const SNDG_ID_SUFFIX_LEN: usize = 4;
pub(crate) const SNDG_MAX_SOUND_FLAG: u32 = 7;

macro_rules! msg {
    ($text:ident, $verbose:ident, $cfg:ident) => {
        if !($cfg.quiet || $verbose > $cfg.verbose) {
            let text = $text.as_ref();
            eprintln!("{text}");
        }
    };
}

pub(crate) fn msg<S: AsRef<str>>(text: S, verbose: u8, cfg: &Cfg, log: &mut Log) -> Result<()> {
    if !cfg.no_log {
        log.write(&text).with_context(|| "Failed to write to log file buffer")?;
    }
    msg!(text, verbose, cfg);
    Ok(())
}

pub(crate) fn msg_no_log<S: AsRef<str>>(text: S, verbose: u8, cfg: &Cfg) {
    msg!(text, verbose, cfg);
}

pub(crate) fn err_or_ignore<S: AsRef<str>>(text: S, ignore: bool, unexpected_tag: bool, cfg: &Cfg, log: &mut Log) -> Result<()> {
    if ignore {
        msg(
            format!("{}{}", cfg.guts.prefix_ignored_important_error_message, text.as_ref()),
            0,
            cfg,
            log,
        )
    } else {
        Err(anyhow!(format!(
            "{}{}{}",
            text.as_ref(),
            if unexpected_tag {
                &cfg.guts.infix_add_unexpected_tag_suggestion
            } else {
                ""
            },
            cfg.guts.suffix_add_ignore_important_errors_suggestion
        )))
    }
}

pub(crate) fn err_or_ignore_thread_safe<S: AsRef<str>>(text: S, ignore: bool, cfg: &Cfg) -> Result<()> {
    if ignore {
        msg_no_log(
            format!("{}{}", cfg.guts.prefix_ignored_important_error_message, text.as_ref()),
            0,
            cfg,
        );
        Ok(())
    } else {
        Err(anyhow!(format!(
            "{}{}",
            text.as_ref(),
            cfg.guts.suffix_add_ignore_important_errors_suggestion
        )))
    }
}

pub(crate) struct Log {
    pub(crate) buffer: Option<BufWriter<File>>,
}

impl Log {
    pub(crate) fn new(cfg: &Cfg) -> Result<Log> {
        if !cfg.no_log {
            let log = match &cfg.log {
                None => return Err(anyhow!("Failed to get log file name")),
                Some(log) => log,
            };
            create_dir_early(log, "log")?;
            let log_backup_message = backup_log_file(log, &cfg.guts.log_backup_suffix);
            let buffer = Some(BufWriter::new(
                File::create(log).with_context(|| format!("Failed to create/open log file \"{}\"", log.display()))?,
            ));
            let mut result = Log { buffer };
            if !log_backup_message.is_empty() {
                msg(log_backup_message, 3, cfg, &mut result)?;
            }
            Ok(result)
        } else {
            Ok(Log { buffer: None })
        }
    }

    pub(crate) fn write<S: AsRef<str>>(&mut self, text: S) -> io::Result<()> {
        match &mut self.buffer {
            None => Ok(()),
            Some(buffer) => {
                writeln!(buffer, "{}", text.as_ref())
            }
        }
    }
}

pub(crate) fn show_log_path(cfg: &Cfg, log: &mut Log) -> Result<()> {
    if cfg.no_log {
        Ok(())
    } else {
        let log_path = match &cfg.log {
            None => return Err(anyhow!("Failed to show log path because it's empty")),
            Some(log_path) => log_path,
        };
        msg(format!("Log is being written into \"{}\"", log_path.display()), 0, cfg, log)
    }
}

pub(crate) fn show_settings_written(cfg: &Cfg, log: &mut Log) -> Result<()> {
    let mut text = String::new();
    if cfg.settings_file.backup_written {
        text.push_str(&format!(
            "Settings file backup was written to \"{}\"{}",
            cfg.settings_file.backup_path.display(),
            if cfg.settings_file.backup_overwritten {
                ", previous backup was overwritten\n"
            } else {
                "\n"
            },
        ))
    }
    text.push_str(&format!(
        "Wrote default program settings into \"{}\"",
        cfg.settings_file.path.display()
    ));
    msg(text, 0, cfg, log)
}

pub(crate) fn create_dir_early(path: &Path, name: &str) -> Result<()> {
    match path.parent() {
        None => {}
        Some(dir) => {
            if dir != Path::new("") && !dir.exists() {
                create_dir_all(dir).with_context(|| format!("Failed to create {} directory \"{}\"", dir.display(), name))?;
                eprintln!(
                    "{} directory \"{}\" was created",
                    name[0..1].to_uppercase() + &name[1..],
                    dir.display()
                )
            }
        }
    }
    Ok(())
}

fn prepare_complex_arg_string(string: &str, pattern: &str, arg_name: &str) -> Result<String> {
    let mut pattern_len = pattern.len();
    let mut string_prepared = &string.to_lowercase().trim().replace('-', "_")[..];
    if let Some(stripped) = string_prepared.strip_prefix("__") {
        pattern_len += 2;
        string_prepared = stripped;
    }
    if string_prepared.starts_with(pattern) {
        Ok(string.trim()[pattern_len..].trim().to_owned())
    } else {
        Err(anyhow!("Error: \"{}\" argument should start with \"{}\"", arg_name, &pattern))
    }
}

pub(crate) fn get_base_dir_path(raw: &str, cfg: &Cfg) -> Result<PathBuf> {
    let base_dir = PathBuf::from(prepare_complex_arg_string(raw, &cfg.guts.list_options_prefix_base_dir, "base_dir")?);
    if base_dir != PathBuf::new() && !base_dir.exists() {
        Err(anyhow!("Error: failed to find base_dir \"{}\"", base_dir.display()))
    } else {
        Ok(base_dir)
    }
}

pub(crate) fn get_game_config_string(raw: &str, cfg: &Cfg) -> Result<String> {
    prepare_complex_arg_string(raw, &cfg.guts.list_options_prefix_config, "config")
}

pub(crate) fn get_append_to_use_load_order_string(raw: &str, cfg: &Cfg) -> Result<String> {
    prepare_complex_arg_string(
        raw,
        &cfg.guts.list_options_prefix_append_to_use_load_order,
        "append_to_use_load_order",
    )
}

pub(crate) fn get_skip_from_use_load_order_string(raw: &str, cfg: &Cfg) -> Result<String> {
    prepare_complex_arg_string(
        raw,
        &cfg.guts.list_options_prefix_skip_from_use_load_order,
        "skip_from_use_load_order",
    )
}

pub(crate) fn references_sorted(references: &mut [&Reference]) {
    references.sort_by_key(|r| {
        (
            // COMMENT: r.moved_cell.is_none(), // openmw 0.47 bug that requires MVRF records to be on top
            // COMMENT: accompany this change with the same change in tes3 library(libs/esp/src/types/cell.rs):
            // COMMENT: reference.moved_cell.is_none(), // openmw 0.47 bug that requires MVRF records to be on top
            !r.persistent(),
            match r.mast_index {
                0 => u32::MAX,
                i => i,
            },
            r.refr_index,
        )
    });
}

pub(crate) fn process_moved_instances(out: &mut Out, h: &mut Helper) -> Result<()> {
    if !h.g.r.moved_instances.is_empty() {
        for (id, grids) in h.g.r.moved_instances.iter() {
            let old_cell_id = match h.g.r.ext_cells.get(&grids.old_grid) {
                None => return Err(anyhow!("Error: failed to find old_cell_id for moved instance")),
                Some(cell_meta) => cell_meta.global_cell_id,
            };
            let new_cell_id = match h.g.r.ext_cells.get(&grids.new_grid) {
                None => return Err(anyhow!("Error: failed to find new_cell_id for moved instance")),
                Some(cell_meta) => cell_meta.global_cell_id,
            };
            let reference = match out.cell[old_cell_id].0.references.remove(id) {
                None => return Err(anyhow!("Error: failed to find moved instance in old cell")),
                Some(reference) => reference,
            };
            let reference_clone = reference.clone();
            if out.cell[new_cell_id]
                .0
                .references
                .insert(
                    *id,
                    Reference {
                        moved_cell: None,
                        ..reference
                    },
                )
                .is_some()
            {
                return Err(anyhow!("Error: new cell already had moved instance"));
            } else if h.g.list_options.turn_normal_grass {
                let old_ref_source = match h.g.r.ext_ref_sources.get_mut(&grids.old_grid) {
                    None => {
                        return Err(anyhow!(
                            "Bug: failed to find cell(old_grid) \"{:?}\" in in ext_ref_sources",
                            &grids.old_grid
                        ))
                    }
                    Some(ext_ref) => match ext_ref.0.remove(id) {
                        None => {
                            return Err(anyhow!(
                                "Bug: failed to properly delete reference \"{:?}\" from cell(old_grid) \"{:?}\" in in ext_ref_sources, it was missing",
                                id,
                                &grids.old_grid
                            ))
                        }
                        Some((old_ref_source, _, _)) => {
                            ext_ref.1.insert(*id, (old_ref_source, reference_clone));
                            old_ref_source
                        }
                    },
                };
                match h.g.r.ext_ref_sources.get_mut(&grids.new_grid) {
                    None => return Err(anyhow!("WTF moved ext_ref_souces")),
                    Some(ext_ref) => ext_ref.0.insert(*id, (old_ref_source, false, true)),
                }
            } else {
                None
            };
        }
    }
    Ok(())
}

pub(crate) fn show_ignored_ref_errors(
    ignored_ref_errors: &[IgnoredRefError],
    plugin_name: &PluginName,
    cell: bool,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    if !ignored_ref_errors.is_empty() {
        let ignored_ref_errors_len = ignored_ref_errors.len();
        let (mut master_suffix, mut cell_suffix, mut ref_suffix, mut encountered_prefix, mut encountered_suffix) =
            ("", "", "", "first ", "(check log for more)");
        if ignored_ref_errors_len > 1 {
            (master_suffix, cell_suffix, ref_suffix) = ("s", "s", "s");
        } else if ignored_ref_errors[0].cell_counter > 1 {
            (cell_suffix, ref_suffix) = ("s", "s")
        } else if ignored_ref_errors[0].ref_counter > 1 {
            ref_suffix = "s"
        } else {
            encountered_prefix = "";
            encountered_suffix = ""
        };
        let cell_msg_part = match cell {
            true => format!("for cell{cell_suffix} "),
            false => String::new(),
        };
        let mut text = format!(
            "Warning: probably outdated plugin \"{plugin_name}\" contains modified cell reference{ref_suffix} {cell_msg_part}missing from master{master_suffix}:"
        );
        for master in ignored_ref_errors {
            text.push_str(&format!(
                "\n  Master \"{}\"({} cell{cell_suffix}, {} ref{ref_suffix}), {encountered_prefix}error encountered was{encountered_suffix}:\n{}",
                master.master, master.cell_counter, master.ref_counter, master.first_encounter,
            ));
        }
        msg(text, 0, cfg, log)?;
    }
    Ok(())
}

pub(crate) fn show_global_list_options(cfg: &Cfg, log: &mut Log) -> Result<()> {
    let text = format!("Global list options: {}", cfg.list_options.show());
    msg(text, 1, cfg, log)
}

pub(crate) fn scan_load_order(h: &mut Helper, cfg: &Cfg, log: &mut Log) -> Result<()> {
    if h.g.config_index == usize::MAX {
        get_game_config(h, cfg, log).with_context(|| "Failed to get game configuration file")?;
    }
    if !h.t.game_configs[h.g.config_index].load_order.scanned {
        get_load_order(h, cfg, log).with_context(|| "Failed to get load order")?;
    }
    Ok(())
}

pub(crate) fn check_presets(h: &mut Helper, cfg: &Cfg, log: &mut Log) -> Result<Vec<Vec<String>>> {
    let mut merge_override: Vec<Vec<String>> = Vec::new();
    if cfg.presets.present {
        h.g.list_options = cfg.list_options.get_list_options(&vec![String::new()][..], cfg, log)?.1;
        if cfg.presets.check_references {
            merge_override = vec![cfg.guts.preset_config_check_references.clone()]
        };
        if cfg.presets.turn_normal_grass {
            let mut preset_config_turn_normal_grass = cfg.guts.preset_config_turn_normal_grass.clone();
            if cfg.presets.check_references {
                preset_config_turn_normal_grass.extend(cfg.guts.preset_config_turn_normal_grass_add_with_check_references.clone());
            }
            merge_override = vec![preset_config_turn_normal_grass];
        };
        if cfg.presets.merge_load_order {
            let mut preset_config_merge_load_order = cfg.guts.preset_config_merge_load_order.clone();
            if cfg.presets.check_references {
                preset_config_merge_load_order.extend(cfg.guts.preset_config_merge_load_order_add_with_check_references.clone());
            }
            if cfg.presets.turn_normal_grass {
                preset_config_merge_load_order.extend(cfg.guts.preset_config_merge_load_order_add_with_turn_normal_grass.clone());
            }
            merge_override = vec![preset_config_merge_load_order];
            scan_load_order(h, cfg, log)?;
            let groundcovers_len = h.t.game_configs[h.g.config_index].load_order.groundcovers.len();
            if groundcovers_len > 0 {
                let mut preset_config_merge_load_order_grass = cfg.guts.preset_config_merge_load_order_grass.clone();
                //
                if cfg.presets.turn_normal_grass {
                    let (_, _, plugin_grass_name) = get_tng_dir_and_plugin_names(&cfg.guts.preset_config_merge_load_order[0], cfg)
                        .with_context(|| "Failed to get turn normal grass directory or plugin names")?;
                    preset_config_merge_load_order_grass.push(format!(
                        "{}{}",
                        cfg.guts.list_options_prefix_append_to_use_load_order, plugin_grass_name
                    ));
                    merge_override.push(preset_config_merge_load_order_grass);
                } else if groundcovers_len > 1 {
                    merge_override.push(preset_config_merge_load_order_grass);
                }
            }
        };
    }
    Ok(merge_override)
}

pub(crate) fn get_expanded_plugin_list(
    plugin_list: &[String],
    index: usize,
    list_options: &ListOptions,
    h: &mut Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<Vec<String>> {
    let expanded_plugin_list = if list_options.use_load_order {
        scan_load_order(h, cfg, log)?;
        let is_grass = matches!(list_options.mode, Mode::Grass);
        if plugin_list.len() > index {
            let text = format!(
                "{} {}plugins defined in list were replaced with contents of load order due to \"use_load_order\" flag",
                plugin_list.len() - index,
                if is_grass { "groundcover " } else { "" },
            );
            msg(text, 0, cfg, log)?;
        } else {
            let text = format!(
                "{} list was expanded with contents of load order due to \"use_load_order\" flag",
                if is_grass { "Groundcover plugins" } else { "Plugin" },
            );
            msg(text, 0, cfg, log)?;
        }
        let mut result = if is_grass {
            plugin_list[..index]
                .iter()
                .cloned()
                .chain(h.t.game_configs[h.g.config_index].load_order.groundcovers.clone())
                .collect::<Vec<_>>()
        } else {
            plugin_list[..index]
                .iter()
                .cloned()
                .chain(h.t.game_configs[h.g.config_index].load_order.contents.clone())
                .collect::<Vec<_>>()
        };
        if !list_options.append_to_use_load_order.is_empty() {
            result.push(list_options.append_to_use_load_order.clone());
            let text = format!(
                "{} list was expanded with \"{}\" due to \"append_to_use_load_order\" option",
                if is_grass { "Groundcover plugins" } else { "Plugin" },
                list_options.append_to_use_load_order
            );
            msg(text, 0, cfg, log)?;
        }
        result
    } else {
        Vec::new()
    };
    Ok(expanded_plugin_list)
}

pub(crate) fn should_skip_list(
    name: &str,
    plugin_list: &[String],
    index: usize,
    list_options: &ListOptions,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<bool> {
    if plugin_list.len() < (index + 1) {
        let text = format!("Output plugin \"{}\" processing skipped due to empty list of plugins", name);
        msg(text, 0, cfg, log)?;
        return Ok(true);
    }
    if !cfg.grass {
        if let Mode::Grass = list_options.mode {
            let text = format!("Output plugin \"{}\" processing skipped due to \"grass=false\"", name);
            msg(text, 0, cfg, log)?;
            return Ok(true);
        };
    }
    Ok(false)
}

pub(crate) fn process_plugin(plugin_name: &str, out: &mut Out, name: &str, h: &mut Helper, cfg: &Cfg, log: &mut Log) -> Result<()> {
    let (plugin_pathbuf, plugin_pathstring) = get_plugin_pathbuf_pathstring(plugin_name, h);
    msg(format!("  Processing plugin \"{}\"", &plugin_pathstring), 2, cfg, log)?;
    h.local_init(plugin_pathbuf, h.g.plugins_processed.len())
        .with_context(|| "Failed to start processing plugin")?;
    let mut plugin: Plugin = Plugin::new();
    plugin
        .load_path(&plugin_pathstring)
        .with_context(|| format!("Failed to load plugin \"{}\"", &plugin_pathstring))?;
    process_records(plugin, out, name, h, cfg, log)
        .with_context(|| format!("Failed to process records from plugin \"{}\"", &plugin_pathstring))?;
    Ok(())
}

fn get_plugin_pathbuf_pathstring(plugin_name: &str, h: &Helper) -> (PathBuf, String) {
    let mut plugin_pathbuf = h.g.list_options.base_dir.clone();
    plugin_pathbuf.push(plugin_name);
    let plugin_path = plugin_pathbuf.to_string_lossy().into_owned();
    (plugin_pathbuf, plugin_path)
}

pub(crate) fn process_turn_normal_grass(
    name: &str,
    out: &mut Out,
    old_plugin: &mut Plugin,
    h: &mut Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    if h.g.list_options.turn_normal_grass {
        let (plugin_deleted_content_name, mut plugin_deleted_content, plugin_grass_name, mut plugin_grass) =
            make_turn_normal_grass(name, out, h, cfg, log)?;
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
            .with_context(|| format!("Failed to write output plugin \"{}\"", plugin_deleted_content_name))?;
        }
        write_output_plugin(&plugin_grass_name, &mut plugin_grass, old_plugin, 2, h, cfg, log)
            .with_context(|| format!("Failed to write output plugin \"{}\"", plugin_grass_name))?;
    }
    Ok(())
}

pub(crate) fn get_tng_dir_and_plugin_names(name: &str, cfg: &Cfg) -> Result<(PathBuf, String, String)> {
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
    let dir = match name_path.parent() {
        None => PathBuf::new(),
        Some(path) => path.to_path_buf(),
    };
    let plugin_deleted_content_name_pathbuf = dir.join(format!(
        "{}{}",
        name_stem, &cfg.guts.turn_normal_grass_plugin_name_suffix_deleted_content
    ));
    let plugin_deleted_content_name = plugin_deleted_content_name_pathbuf.to_string_lossy().into_owned();
    let plugin_grass_name_pathbuf = dir.join(format!("{}{}", name_stem, &cfg.guts.turn_normal_grass_plugin_name_suffix_grass));
    let plugin_grass_name = plugin_grass_name_pathbuf.to_string_lossy().into_owned();
    Ok((dir, plugin_deleted_content_name, plugin_grass_name))
}

pub(crate) fn read_lines(filename: &Path) -> Result<io::Lines<io::BufReader<File>>> {
    let file = File::open(filename).with_context(|| format!("Failed to open file \"{}\"", filename.display()))?;
    Ok(io::BufReader::new(file).lines())
}

pub(crate) fn show_settings_version_message(cfg: &Cfg, log: &mut Log) -> Result<()> {
    if let Some(message) = &cfg.settings_file.version_message {
        msg(message, 0, cfg, log)
    } else {
        Ok(())
    }
}

fn backup_log_file(log_file: &PathBuf, backup_suffix: &str) -> String {
    let mut backup_path = log_file.clone().into_os_string();
    backup_path.push(backup_suffix);
    let backup_file: PathBuf = backup_path.into();
    match copy(log_file, &backup_file) {
        Ok(_) => format!("Previous log file was saved to \"{}\"", backup_file.display()),
        Err(_) => String::new(),
    }
}

pub(crate) fn get_cell_name(cell: &Cell) -> String {
    if cell.data.flags.contains(CellFlags::IS_INTERIOR) {
        format!("{:?}", cell.name)
    } else {
        format!(
            "\"{}{:?}\"",
            if let Some(region) = &cell.region {
                format!("{} ", region)
            } else {
                String::new()
            },
            cell.data.grid
        )
    }
}

pub(crate) fn show_removed_record_ids(
    removed_record_ids: Vec<String>,
    reason: &str,
    name: &str,
    verbosity: u8,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    if !removed_record_ids.is_empty() {
        if verbosity < 1 {
            return Err(anyhow!(
                "Bug: verbosity passed to show_removed_record_ids should be >= 1, value passed is \"{}\"",
                verbosity
            ));
        }
        let removed_record_ids_len = removed_record_ids.len();
        let mut text = format!(
            "  {} record{} excluded from \"{}\" due to {}",
            removed_record_ids_len,
            if removed_record_ids_len == 1 { " was" } else { "s were" },
            name,
            reason
        );
        if cfg.verbose < verbosity {
            msg_no_log(
                format!(
                    "{}{}",
                    &text,
                    &format!("(check log or add -{} to get list)", "v".repeat(verbosity.into()))
                ),
                0,
                cfg,
            );
        }
        text.push_str(":\n");
        text.push_str(&removed_record_ids.join("\n"));
        msg(text, verbosity, cfg, log)
    } else {
        Ok(())
    }
}

pub(crate) fn select_header_description(h: &Helper, cfg: &Cfg) -> String {
    let len = h.g.plugins_processed.len();
    match len {
        1 => format!(
            "{}{}{}",
            &cfg.guts.header_description_processed_one_plugin_prefix,
            h.g.plugins_processed[0].name,
            &cfg.guts.header_description_processed_one_plugin_suffix
        ),
        _ => format!(
            "{}{}{}",
            &cfg.guts.header_description_merged_many_plugins_prefix, len, &cfg.guts.header_description_merged_many_plugins_suffix
        ),
    }
}

pub(crate) fn truncate_header_text(field: &str, len: usize, value: &str, cfg: &Cfg, log: &mut Log) -> Result<String> {
    if value.len() > len {
        let truncated_value = &value[..len];
        let mut text = format!(
            "Warning: header's \"{}\" field was truncated to \"{}\" characters(format's limit for this field)",
            field, len
        );
        msg_no_log(format!("{}, check log for details", text), 0, cfg);
        text.push_str(&format!(
            ":\n  Original value was:\n    \"{}\"\n  Truncated value is:\n    \"{}\"\n  Characters cut({}):\n    \"{}\"",
            value,
            truncated_value,
            value.len() - len,
            &value[len..]
        ));
        msg(&text, u8::MAX, cfg, log)?;
        Ok(truncated_value[..len].to_owned())
    } else {
        Ok(value.to_owned())
    }
}

pub(crate) fn get_tng_content_name_low(name: &str, h: &Helper, cfg: &Cfg) -> Result<String> {
    if !h.g.list_options.turn_normal_grass && !h.g.list_options.use_load_order {
        Ok(String::new())
    } else {
        let (_, tng_content_name, _) =
            get_tng_dir_and_plugin_names(name, cfg).with_context(|| "Failed to get turn normal grass directory or plugin names")?;
        Ok(tng_content_name.to_lowercase())
    }
}

pub(crate) fn get_skip_plugin_name_low(h: &Helper) -> String {
    if h.g.list_options.use_load_order && !h.g.list_options.skip_from_use_load_order.is_empty() {
        h.g.list_options.skip_from_use_load_order.to_lowercase()
    } else {
        String::new()
    }
}
