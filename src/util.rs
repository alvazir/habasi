use crate::{
    input, load_order, make_turn_normal_grass, write_output_plugin, Cfg, Helper, IgnoredRefError,
    ListOptions, Mode, Out, Plugin, PluginName, RegexPluginInfo,
};
use anyhow::{anyhow, Context, Result};
use crc::{Crc, CRC_64_ECMA_182};
use fs_err::{create_dir_all, metadata, read_dir, rename, File};
use glob::{glob_with, MatchOptions};
use regex::RegexBuilder;
use std::{
    fmt::Write as _,
    io::{self, BufRead, BufWriter, Write},
    path::{Path, PathBuf, MAIN_SEPARATOR},
    time::SystemTime,
};
use tes3::esp::{Cell, CellFlags, Reference};

pub const CRC64: Crc<u64> = Crc::<u64>::new(&CRC_64_ECMA_182);
pub const SNDG_ID_MAX_LEN: usize = 32;
pub const SNDG_ID_SUFFIX_LEN: usize = 4;
pub const SNDG_MAX_SOUND_FLAG: u32 = 7;

macro_rules! msg {
    ($text:ident, $verbose:ident, $cfg:ident) => {
        if !($cfg.quiet || $verbose > $cfg.verbose) {
            let text = $text.as_ref();
            eprintln!("{text}");
        }
    };
}

pub fn msg<S: AsRef<str>>(text: S, verbose: u8, cfg: &Cfg, log: &mut Log) -> Result<()> {
    if !cfg.no_log {
        log.write(&text)
            .with_context(|| "Failed to write to log file buffer")?;
    }
    msg!(text, verbose, cfg);
    Ok(())
}

pub fn msg_no_log<S: AsRef<str>>(text: S, verbose: u8, cfg: &Cfg) {
    msg!(text, verbose, cfg);
}

pub fn err_or_ignore<S: AsRef<str>>(
    text: S,
    ignore: bool,
    unexpected_tag: bool,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    if ignore {
        msg(
            format!(
                "{}{}",
                cfg.guts.prefix_ignored_important_error_message,
                text.as_ref()
            ),
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

pub fn err_or_ignore_thread_safe<S: AsRef<str>>(text: S, ignore: bool, cfg: &Cfg) -> Result<()> {
    if ignore {
        msg_no_log(
            format!(
                "{}{}",
                cfg.guts.prefix_ignored_important_error_message,
                text.as_ref()
            ),
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

pub struct Log {
    pub(crate) buffer: Option<BufWriter<File>>,
}

impl Log {
    pub(crate) fn new(cfg: &Cfg) -> Result<Self> {
        if cfg.no_log {
            Ok(Self { buffer: None })
        } else {
            let log = match cfg.log {
                None => return Err(anyhow!("Failed to get log file name")),
                Some(ref log) => log,
            };
            create_dir_early(log, "Log")?;
            let log_backup_message = backup_log_file(log, &cfg.guts.log_backup_suffix);
            let buffer = Some(BufWriter::new(File::create(log).with_context(|| {
                format!("Failed to create/open log file \"{}\"", log.display())
            })?));
            let mut result = Self { buffer };
            if !log_backup_message.is_empty() {
                msg(log_backup_message, 3, cfg, &mut result)?;
            }
            Ok(result)
        }
    }

    pub(crate) fn write<S: AsRef<str>>(&mut self, text: S) -> io::Result<()> {
        self.buffer
            .as_mut()
            .map_or_else(|| Ok(()), |buffer| writeln!(buffer, "{}", text.as_ref()))
    }
}

pub fn show_log_path(cfg: &Cfg, log: &mut Log) -> Result<()> {
    if cfg.no_log {
        Ok(())
    } else {
        let log_path = match cfg.log {
            None => return Err(anyhow!("Failed to show log path because it's empty")),
            Some(ref log_path) => log_path,
        };
        msg(
            format!("Log is written to \"{}\"", log_path.display()),
            0,
            cfg,
            log,
        )
    }
}

pub fn show_settings_written(cfg: &Cfg, log: &mut Log) -> Result<()> {
    let mut text = String::new();
    if cfg.settings_file.backup_written {
        write!(
            text,
            "Previous settings file was renamed to \"{}\"{}",
            cfg.settings_file.backup_path.display(),
            if cfg.settings_file.backup_overwritten {
                ", previous backup was overwritten\n"
            } else {
                "\n"
            },
        )?;
    }
    write!(
        text,
        "Wrote default program settings into \"{}\"",
        cfg.settings_file.path.display()
    )?;
    msg(text, 0, cfg, log)
}

pub fn create_dir_early(path: &Path, name_capitalized: &str) -> Result<()> {
    if let Some(dir) = path.parent() {
        #[allow(clippy::print_stderr)]
        if dir != Path::new("") && !dir.exists() {
            create_dir_all(dir).with_context(|| {
                format!(
                    "Failed to create {} directory \"{}\"",
                    dir.display(),
                    name_capitalized.to_lowercase()
                )
            })?;
            eprintln!(
                "{name_capitalized} directory \"{}\" was created",
                dir.display()
            );
        }
    }
    Ok(())
}

fn prepare_complex_arg_string(string: &str, pattern: &str, arg_name: &str) -> Result<String> {
    let mut pattern_len = pattern.len();
    let mut string_prepared = &*string.to_lowercase().trim().replace('-', "_");
    let long_prefix = "__";
    if let Some(stripped) = string_prepared.strip_prefix(long_prefix) {
        pattern_len = pattern_len
            .checked_add(long_prefix.len())
            .with_context(|| {
                format!(
                    "Bug: overflow adding \"{}\" to \"{pattern_len}\"",
                    long_prefix.len()
                )
            })?;
        string_prepared = stripped;
    }
    if string_prepared.starts_with(pattern) {
        Ok(string
            .trim()
            .get(pattern_len..)
            .with_context(|| {
                format!("Bug: argument \"{string}\" contains nothing after pattern \"{pattern}\"")
            })?
            .trim()
            .to_owned())
    } else {
        Err(anyhow!(
            "Error: \"{}\" argument should start with \"{}\"",
            arg_name,
            &pattern
        ))
    }
}

pub fn get_base_dir_path(raw: &str, cfg: &Cfg) -> Result<PathBuf> {
    let base_dir = PathBuf::from(prepare_complex_arg_string(
        raw,
        &cfg.guts.list_options_prefix_base_dir,
        "base_dir",
    )?);
    if base_dir != PathBuf::new() && !base_dir.exists() {
        Err(anyhow!(
            "Error: failed to find base_dir \"{}\"",
            base_dir.display()
        ))
    } else {
        Ok(base_dir)
    }
}

pub fn get_game_config_string(raw: &str, cfg: &Cfg) -> Result<String> {
    prepare_complex_arg_string(raw, &cfg.guts.list_options_prefix_config, "config")
}

pub fn get_append_to_use_load_order_string(raw: &str, cfg: &Cfg) -> Result<String> {
    prepare_complex_arg_string(
        raw,
        &cfg.guts.list_options_prefix_append_to_use_load_order,
        "append_to_use_load_order",
    )
}

pub fn get_skip_from_use_load_order_string(raw: &str, cfg: &Cfg) -> Result<String> {
    prepare_complex_arg_string(
        raw,
        &cfg.guts.list_options_prefix_skip_from_use_load_order,
        "skip_from_use_load_order",
    )
}

pub fn references_sorted(references: &mut [&Reference]) {
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

pub fn process_moved_instances(out: &mut Out, h: &mut Helper) -> Result<()> {
    if !h.g.r.moved_instances.is_empty() {
        for (id, grids) in &h.g.r.moved_instances {
            let old_cell_id = match h.g.r.ext_cells.get(&grids.old_grid) {
                None => {
                    return Err(anyhow!(
                        "Error: failed to find old_cell_id for moved instance"
                    ))
                }
                Some(cell_meta) => cell_meta.global_cell_id,
            };
            let new_cell_id = match h.g.r.ext_cells.get(&grids.new_grid) {
                None => {
                    return Err(anyhow!(
                        "Error: failed to find new_cell_id for moved instance"
                    ))
                }
                Some(cell_meta) => cell_meta.global_cell_id,
            };
            let reference = match out
                .cell
                .get_mut(old_cell_id)
                .with_context(|| {
                    format!("Bug: out.cell with old_cell_id = \"{old_cell_id}\" not found")
                })?
                .0
                .references
                .remove(id)
            {
                None => return Err(anyhow!("Error: failed to find moved instance in old cell")),
                Some(reference) => reference,
            };
            let reference_clone = reference.clone();
            if out
                .cell
                .get_mut(new_cell_id)
                .with_context(|| {
                    format!("Bug: out.cell with new_cell_id = \"{new_cell_id}\" not found")
                })?
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

pub fn show_ignored_ref_errors(
    ignored_ref_errors: &[IgnoredRefError],
    plugin_name: &PluginName,
    cell: bool,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    if !ignored_ref_errors.is_empty() {
        let ignored_ref_errors_len = ignored_ref_errors.len();
        let (
            mut master_suffix,
            mut cell_suffix,
            mut ref_suffix,
            mut encountered_prefix,
            mut encountered_suffix,
        ) = ("", "", "", "first ", "(check log for more)");
        if ignored_ref_errors_len > 1 {
            master_suffix = "s";
            cell_suffix = "s";
            ref_suffix = "s";
        } else {
            let ignored_ref_errors_first = ignored_ref_errors
                .first()
                .with_context(|| "Bug: ignored_ref_errors is empty")?;
            if ignored_ref_errors_first.cell_counter > 1 {
                cell_suffix = "s";
                ref_suffix = "s";
            } else if ignored_ref_errors_first.ref_counter > 1 {
                ref_suffix = "s";
            } else {
                encountered_prefix = "";
                encountered_suffix = "";
            }
        };
        let cell_msg_part = if cell {
            format!("for cell{cell_suffix} ")
        } else {
            String::new()
        };
        let mut text = format!(
            "Warning: probably outdated plugin \"{plugin_name}\" contains modified cell reference{ref_suffix} {cell_msg_part}missing from master{master_suffix}:"
        );
        for master in ignored_ref_errors {
            write!(text,
                "\n  Master \"{}\"({} cell{cell_suffix}, {} ref{ref_suffix}), {encountered_prefix}error encountered was{encountered_suffix}:\n{}",
                master.master, master.cell_counter, master.ref_counter, master.first_encounter,
            )?;
        }
        msg(text, 0, cfg, log)?;
    }
    Ok(())
}

pub fn show_global_list_options(cfg: &Cfg, log: &mut Log) -> Result<()> {
    let text = format!("Global list options: {}", cfg.list_options.show()?);
    msg(text, 1, cfg, log)
}

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
            merge_override = vec![preset_config_merge_load_order];
            load_order::scan(h, cfg, log)?;
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

pub fn get_expanded_plugin_list(
    plugin_list: &[String],
    index: usize,
    list_options: &ListOptions,
    h: &mut Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<Vec<String>> {
    let expanded_plugin_list = if list_options.use_load_order {
        h.g.list_options = list_options.get_pristine();
        load_order::scan(h, cfg, log)?;
        let is_grass = matches!(list_options.mode, Mode::Grass);
        if plugin_list.len() > index {
            #[allow(clippy::arithmetic_side_effects)]
            let text = format!(
                "{} {}plugins defined in list were replaced with contents of load order due to \"use_load_order\" flag",
                plugin_list.len() - index,
                if is_grass { "groundcover " } else { "" },
            );
            msg(text, 0, cfg, log)?;
        } else {
            let text =
                format!(
                "{} list was expanded with contents of load order due to \"use_load_order\" flag",
                if is_grass { "Groundcover plugins" } else { "Plugin" },
            );
            msg(text, 0, cfg, log)?;
        }
        macro_rules! result {
            ($kind:ident) => {
                plugin_list
                    .get(..index)
                    .with_context(|| {
                        format!(
                            "Bug: plugin_list.len() = \"{}\" < index = \"{index}\"",
                            plugin_list.len()
                        )
                    })?
                    .iter()
                    .cloned()
                    .chain(
                        h.t.game_configs
                            .get(h.g.config_index)
                            .with_context(|| {
                                format!(
                                    "Bug: h.t.game_configs doesn't contain h.g.config_index = \"{}\"",
                                    h.g.config_index
                                )
                            })?
                            .load_order
                            .$kind
                            .clone(),
                    )
                    .collect::<Vec<_>>()
            };
        }
        let mut result = if is_grass {
            result!(groundcovers)
        } else {
            result!(contents)
        };
        if !list_options.append_to_use_load_order.is_empty() {
            result.push(list_options.append_to_use_load_order.clone());
            let text = format!(
                "{} list was expanded with \"{}\" due to \"append_to_use_load_order\" option",
                if is_grass {
                    "Groundcover plugins"
                } else {
                    "Plugin"
                },
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

pub fn should_skip_list(
    name: &str,
    plugin_list: &[String],
    index: usize,
    list_options: &ListOptions,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<bool> {
    let text = if plugin_list.len() <= index {
        format!("Output plugin {name:?} processing skipped due to empty list of plugins")
    } else if !cfg.grass && matches!(list_options.mode, Mode::Grass) {
        format!("Output plugin {name:?} processing skipped due to \"grass=false\"")
    } else {
        return Ok(false);
    };
    msg(text, 0, cfg, log)?;
    Ok(true)
}

pub fn process_plugin(
    plugin_name: &str,
    out: &mut Out,
    name: &str,
    h: &mut Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let (plugin_pathbuf, plugin_pathstring) = get_plugin_pathbuf_pathstring(plugin_name, h);
    msg(
        format!("  Processing plugin \"{}\"", &plugin_pathstring),
        2,
        cfg,
        log,
    )?;
    h.local_init(plugin_pathbuf, h.g.plugins_processed.len())
        .with_context(|| "Failed to start processing plugin")?;
    let mut plugin = Plugin::new();
    plugin
        .load_path(&plugin_pathstring)
        .with_context(|| format!("Failed to load plugin \"{}\"", &plugin_pathstring))?;
    input::process_records(plugin, out, name, h, cfg, log).with_context(|| {
        format!(
            "Failed to process records from plugin \"{}\"",
            &plugin_pathstring
        )
    })?;
    Ok(())
}

fn get_plugin_pathbuf_pathstring(plugin_name: &str, h: &Helper) -> (PathBuf, String) {
    let mut plugin_pathbuf = h.g.list_options.base_dir.clone();
    plugin_pathbuf.push(plugin_name);
    let plugin_path = plugin_pathbuf.to_string_lossy().into_owned();
    (plugin_pathbuf, plugin_path)
}

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
        name_stem,
        &cfg.guts
            .turn_normal_grass_plugin_name_suffix_deleted_content
    ));
    let plugin_deleted_content_name = plugin_deleted_content_name_pathbuf
        .to_string_lossy()
        .into_owned();
    let plugin_grass_name_pathbuf = dir.join(format!(
        "{}{}",
        name_stem, &cfg.guts.turn_normal_grass_plugin_name_suffix_grass
    ));
    let plugin_grass_name = plugin_grass_name_pathbuf.to_string_lossy().into_owned();
    Ok((dir, plugin_deleted_content_name, plugin_grass_name))
}

pub fn read_lines(filename: &Path) -> Result<io::Lines<io::BufReader<File>>> {
    let file = File::open(filename)
        .with_context(|| format!("Failed to open file \"{}\"", filename.display()))?;
    Ok(io::BufReader::new(file).lines())
}

pub fn show_settings_version_message(cfg: &Cfg, log: &mut Log) -> Result<()> {
    cfg.settings_file
        .version_message
        .as_ref()
        .map_or_else(|| Ok(()), |message| msg(message, 0, cfg, log))
}

fn backup_log_file(log_file: &PathBuf, backup_suffix: &str) -> String {
    let mut backup_path = log_file.clone().into_os_string();
    backup_path.push(backup_suffix);
    let backup_file: PathBuf = backup_path.into();
    match rename(log_file, &backup_file) {
        Ok(()) => format!(
            "Previous log file was renamed to \"{}\"",
            backup_file.display()
        ),
        Err(_) => String::new(),
    }
}

pub fn get_cell_name(cell: &Cell) -> String {
    if cell.data.flags.contains(CellFlags::IS_INTERIOR) {
        format!("{:?}", cell.name)
    } else {
        format!(
            "\"{}{:?}\"",
            cell.region
                .as_ref()
                .map_or_else(String::new, |region| format!("{region} ")),
            cell.data.grid
        )
    }
}

pub fn show_removed_record_ids(
    removed_record_ids: &[String],
    reason: &str,
    name: &str,
    verbosity: u8,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    if removed_record_ids.is_empty() {
        Ok(())
    } else {
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
            if removed_record_ids_len == 1 {
                " was"
            } else {
                "s were"
            },
            name,
            reason
        );
        if cfg.verbose < verbosity {
            msg_no_log(
                format!(
                    "{}{}",
                    &text,
                    &format!(
                        "(check log or add -{} to get list)",
                        "v".repeat(verbosity.into())
                    )
                ),
                0,
                cfg,
            );
        }
        text.push_str(":\n");
        text.push_str(&removed_record_ids.join("\n"));
        msg(text, verbosity, cfg, log)
    }
}

pub fn select_header_description(h: &Helper, cfg: &Cfg) -> String {
    let len = h.g.plugins_processed.len();
    if len == 1 {
        format!(
            "{}{}{}",
            &cfg.guts.header_description_processed_one_plugin_prefix,
            h.g.plugins_processed
                .first()
                .map_or("", |plugin_processed| &plugin_processed.name),
            &cfg.guts.header_description_processed_one_plugin_suffix
        )
    } else {
        format!(
            "{}{}{}",
            &cfg.guts.header_description_merged_many_plugins_prefix,
            len,
            &cfg.guts.header_description_merged_many_plugins_suffix
        )
    }
}

pub fn truncate_header_text(
    field: &str,
    len: usize,
    value: &str,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<String> {
    #[allow(clippy::arithmetic_side_effects)]
    if value.len() > len {
        let truncated_value = value.get(..len).with_context(|| "Bug: indexing slicing")?;
        let mut text = format!("Warning: header's {field:?} field was truncated to {len:?} characters(format's limit for this field)");
        msg_no_log(format!("{text}, check log for details"), 0, cfg);
        write!(
            text,
            ":\n  Original value was:\n    \"{}\"\n  Truncated value is:\n    \"{}\"\n  Characters cut({}):\n    \"{}\"",
            value,
            truncated_value,
            value.len() - len,
            &value.get(len..).with_context(|| "Bug: indexing slicing")?
        )?;
        msg(&text, u8::MAX, cfg, log)?;
        Ok(truncated_value
            .get(..len)
            .with_context(|| "Bug: indexing slicing")?
            .to_owned())
    } else {
        Ok(value.to_owned())
    }
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

pub fn get_skip_plugin_name_low(h: &Helper) -> String {
    if h.g.list_options.use_load_order && !h.g.list_options.skip_from_use_load_order.is_empty() {
        h.g.list_options.skip_from_use_load_order.to_lowercase()
    } else {
        String::new()
    }
}

pub fn get_regex_plugin_list(
    plugin_list: &[String],
    index: usize,
    list_options: &ListOptions,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<Vec<String>> {
    let mut regex_plugin_list = Vec::new();
    let regex_sublists = get_regex_sublists(plugin_list, index, list_options, cfg, log)?;
    if !regex_sublists.is_empty() {
        regex_plugin_list = plugin_list
            .get(..index)
            .with_context(|| format!("Bug: indexing slicing plugin_list[..{index}]"))?
            .iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let mut new_index = index;
        let mut sum: usize;
        for (subindex, sublist) in regex_sublists {
            sum = subindex.checked_add(index).with_context(|| {
                format!("Bug: overflow adding index = \"{index}\" to subindex = \"{subindex}\"")
            })?;
            if sum > new_index {
                regex_plugin_list.extend(
                    plugin_list
                        .get(new_index..sum)
                        .with_context(|| {
                            format!("Bug: indexing slicing plugin_list[{new_index}..{sum}]")
                        })?
                        .iter()
                        .map(ToOwned::to_owned),
                );
                new_index = sum;
            }
            if !sublist.is_empty() {
                regex_plugin_list.extend(sublist.into_iter());
            }
            new_index = new_index.checked_add(1).with_context(|| {
                format!("Bug: overflow incrementing new_index = \"{new_index}\"")
            })?;
        }
        if new_index < plugin_list.len() {
            regex_plugin_list.extend(
                plugin_list
                    .get(new_index..)
                    .with_context(|| format!("Bug: indexing slicing plugin_list[{new_index}..]"))?
                    .iter()
                    .map(ToOwned::to_owned),
            );
        }
    }
    Ok(regex_plugin_list)
}

fn get_regex_sublists(
    plugin_list: &[String],
    index: usize,
    list_options: &ListOptions,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<Vec<(usize, Vec<String>)>> {
    let mut regex_sublists = Vec::new();
    let mut sublist = Vec::new();
    let mut split: Vec<&str>;
    for (subindex, item) in plugin_list
        .get(index..)
        .with_context(|| format!("Bug: indexing slicing plugin_list[{index}..]"))?
        .iter()
        .enumerate()
    {
        split = item.splitn(2, ':').collect();
        if split.len() != 2 {
            continue;
        }
        #[allow(clippy::indexing_slicing)]
        let (pattern, is_regex) = if split[0].to_lowercase() == "regex" {
            (split[1], true)
        } else if split[0].to_lowercase() == "glob" {
            (split[1], false)
        } else {
            continue;
        };
        if pattern.is_empty() {
            let text = format!("Pattern is empty in argument: {item:?}");
            err_or_ignore(text, list_options.ignore_important_errors, false, cfg, log)?;
            regex_sublists.push((subindex, Vec::new()));
            continue;
        }
        let mut sort_by_name = list_options.regex_sort_by_name;
        let mut plugin_pathbuf = list_options.base_dir.clone();
        plugin_pathbuf.push(pattern);
        let mut remove_leading_dot = false;
        sublist.clear();
        if let Err(error) = if is_regex {
            get_regex_plugins(
                &mut sublist,
                &plugin_pathbuf,
                &mut sort_by_name,
                &mut remove_leading_dot,
                list_options,
                cfg,
                log,
            )
        } else {
            get_glob_plugins(
                &mut sublist,
                &plugin_pathbuf,
                &mut sort_by_name,
                list_options,
                cfg,
                log,
            )
        } {
            err_or_ignore(
                format!("{error:?}"),
                list_options.ignore_important_errors,
                false,
                cfg,
                log,
            )
            .with_context(|| {
                format!(
                    "Failed to get plugins from {} pattern: {pattern:?}",
                    if is_regex { "regex" } else { "glob" }
                )
            })?;
            regex_sublists.push((subindex, Vec::new()));
            continue;
        };
        if sort_by_name {
            sublist.sort_by(|a, b| a.name_low.cmp(&b.name_low).then(a.path.cmp(&b.path)));
        } else {
            sublist.sort_by(|a, b| a.time.cmp(&b.time).then(a.path.cmp(&b.path)));
        }
        let regex_sublist = get_regex_sublist(&sublist, remove_leading_dot, list_options);
        if regex_sublist.is_empty() {
            let text = format!("Nothing found for pattern: {pattern:?}");
            err_or_ignore(text, list_options.ignore_important_errors, false, cfg, log)?;
        } else {
            let mut text = format!("Pattern {item:?} expanded to:");
            for plugin in &regex_sublist {
                if plugin.contains(' ') {
                    write!(text, " \"{plugin}\"")?;
                } else {
                    write!(text, " {plugin}")?;
                };
            }
            msg(&text, 0, cfg, log)?;
        }
        regex_sublists.push((subindex, regex_sublist));
    }
    Ok(regex_sublists)
}

fn get_regex_sublist(
    sublist: &[RegexPluginInfo],
    remove_leading_dot: bool,
    list_options: &ListOptions,
) -> Vec<String> {
    let prefix = if remove_leading_dot {
        format!(".{MAIN_SEPARATOR}")
    } else if list_options.base_dir != PathBuf::new() {
        format!(
            "{}{MAIN_SEPARATOR}",
            list_options.base_dir.to_string_lossy()
        )
    } else {
        String::new()
    };
    sublist
        .iter()
        .map(|regex_plugin_info| regex_plugin_info.path.to_string_lossy())
        .map(|path_str| {
            if prefix.is_empty() {
                path_str.into_owned()
            } else if let Some(stripped) = path_str.strip_prefix(&prefix) {
                stripped.to_owned()
            } else {
                path_str.into_owned()
            }
        })
        .collect::<Vec<String>>()
}

fn get_plugin_time(
    path: &Path,
    sort_by_name: &mut bool,
    pattern: &Path,
    pattern_kind: &str,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<SystemTime> {
    let time = if *sort_by_name {
        SystemTime::now()
    } else {
        match metadata(path) {
            Ok(meta) => match meta.modified() {
                Ok(time) => time,
                Err(error) => {
                    let text = format!(
                    "Falling back to \"--sort-by-name\" for the {pattern_kind} {pattern:?} because failed to get file modification time for {path:?} with error: {error:?}"
                );
                    msg(&text, 0, cfg, log)?;
                    *sort_by_name = false;
                    SystemTime::now()
                }
            },
            Err(error) => {
                let text = format!(
                    "Falling back to \"--sort-by-name\" for the {pattern_kind} {pattern:?} because failed to get file metadata for {path:?} with error: {error:?}"
                );
                msg(&text, 0, cfg, log)?;
                *sort_by_name = false;
                SystemTime::now()
            }
        }
    };
    Ok(time)
}

fn get_regex_plugins(
    list: &mut Vec<RegexPluginInfo>,
    pattern: &Path,
    sort_by_name: &mut bool,
    remove_leading_dot: &mut bool,
    list_options: &ListOptions,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let mut dir = Path::new(&pattern);
    loop {
        match dir.parent() {
            None => break,
            Some(parent) => {
                dir = parent;
                if parent.is_dir() {
                    break;
                }
            }
        }
    }
    let regex_pattern = pattern
        .to_string_lossy()
        .strip_prefix(&format!("{}{}", dir.to_string_lossy(), MAIN_SEPARATOR))
        .map_or_else(|| pattern.to_string_lossy().into_owned(), ToOwned::to_owned);
    let regex_expression = RegexBuilder::new(&regex_pattern)
        .case_insensitive(!list_options.regex_case_sensitive)
        .build()?;
    if dir == Path::new("") {
        *remove_leading_dot = true;
        dir = Path::new(".");
    };
    for entry in read_dir(dir)?.flatten() {
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
                    && regex_expression.is_match(&entry.file_name().to_string_lossy())
                {
                    let time = get_plugin_time(&path, sort_by_name, pattern, "regex", cfg, log)
                        .with_context(|| {
                            format!("Failed to get modification time for: {path:?}")
                        })?;
                    let name_low = entry.file_name().to_string_lossy().to_lowercase();
                    list.push(RegexPluginInfo {
                        path,
                        name_low,
                        time,
                    });
                }
            }
        }
    }
    Ok(())
}

fn get_glob_plugins(
    list: &mut Vec<RegexPluginInfo>,
    pattern: &Path,
    sort_by_name: &mut bool,
    list_options: &ListOptions,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let glob_options = MatchOptions {
        case_sensitive: list_options.regex_case_sensitive,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };
    for path in glob_with(&pattern.to_string_lossy(), glob_options)?.flatten() {
        let name_low = match path.file_name() {
            Some(osstr) => osstr.to_string_lossy().to_lowercase(),
            None => continue,
        };
        if let Some(plugin_extension) = path.extension() {
            if cfg
                .guts
                .omw_plugin_extensions
                .contains(&plugin_extension.to_ascii_lowercase())
            {
                let time = get_plugin_time(&path, sort_by_name, pattern, "glob", cfg, log)
                    .with_context(|| format!("Failed to get modification time for: {path:?}"))?;
                list.push(RegexPluginInfo {
                    path,
                    name_low,
                    time,
                });
            }
        };
    }
    Ok(())
}
