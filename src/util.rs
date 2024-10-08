use crate::{input::process_records, Cfg, Helper, ListOptions, Mode, Out, Plugin};
use anyhow::{anyhow, Context, Result};
use crc::{Crc, CRC_64_ECMA_182};
use fs_err::{create_dir_all, File};
use std::{
    fmt::Write as _,
    io::{BufRead, BufReader, Lines},
    path::{Path, PathBuf},
};
use tes3::esp::{Cell, CellFlags};
pub mod header;
pub mod load_order;
pub mod log;
pub mod msg;
pub mod patterns;
pub mod presets;
pub mod references;
pub mod tng;
use log::Log;
use msg::{err_or_ignore, msg, msg_no_log};

pub const CRC64: Crc<u64> = Crc::<u64>::new(&CRC_64_ECMA_182);
pub const SNDG_ID_MAX_LEN: usize = 32;
pub const SNDG_ID_SUFFIX_LEN: usize = 4;
pub const SNDG_MAX_SOUND_FLAG: u32 = 7;

macro_rules! increment {
    ($($field:ident).+) => {
        $($field.)+checked_add(1).with_context(|| {
            format!(
                "Bug: overflow incrementing {} = \"{}\"",
                stringify!($($field).+),
                $($field).+
            )
        })?
    };
}

pub(crate) use increment;

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
    if !base_dir.as_os_str().is_empty() && !base_dir.exists() {
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

pub fn show_global_list_options(cfg: &Cfg, log: &mut Log) -> Result<()> {
    let text = format!("Global list options: {}", cfg.list_options.show()?);
    msg(text, 1, cfg, log)
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
    let plugin = Plugin::from_path(&plugin_pathstring)
        .with_context(|| format!("Failed to read plugin \"{plugin_pathstring}\""))?;
    process_records(plugin, out, name, h, cfg, log).with_context(|| {
        format!(
            "Failed to process records from plugin \"{}\"",
            &plugin_pathstring
        )
    })?;
    Ok(())
}

fn get_plugin_pathbuf_pathstring(plugin_name: &str, h: &Helper) -> (PathBuf, String) {
    let mut plugin_pathbuf = h.g.list_options.indirect.base_dir.clone();
    plugin_pathbuf.push(plugin_name);
    let plugin_path = plugin_pathbuf.to_string_lossy().into_owned();
    (plugin_pathbuf, plugin_path)
}

pub fn read_lines(filename: &Path) -> Result<Lines<BufReader<File>>> {
    let file = File::open(filename)
        .with_context(|| format!("Failed to open file \"{}\"", filename.display()))?;
    Ok(BufReader::new(file).lines())
}

pub fn show_settings_version_message(cfg: &Cfg, log: &mut Log) -> Result<()> {
    cfg.settings_file
        .version_message
        .as_ref()
        .map_or_else(|| Ok(()), |message| msg(message, 0, cfg, log))
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
