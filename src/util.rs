use crate::{Cfg, Helper, IgnoredRefError, Mode, Out, PluginNameLow};
use anyhow::{anyhow, Context, Result};
use std::{
    fs::{create_dir_all, File},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};
use tes3::esp::Reference;

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
            let buffer = Some(BufWriter::new(
                File::create(log).with_context(|| format!("Failed to create/open log file \"{}\"", log.display()))?,
            ));
            Ok(Log { buffer })
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
    msg(
        format!("Wrote default program settings into \"{}\"", cfg.settings.display()),
        0,
        cfg,
        log,
    )
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

pub(crate) fn get_base_dir(base_dir_string: &str) -> Result<PathBuf> {
    let base_dir_low = &base_dir_string.to_lowercase()[..];
    let base_dir = match base_dir_low {
        "base_dir:off" => PathBuf::new(),
        _ => match &base_dir_string.strip_prefix("base_dir:") {
            Some(path) => PathBuf::from(path.trim()),
            None => return Err(anyhow!("Error: base_dir argument should start with \"base_dir:\"")),
        },
    };
    if base_dir != PathBuf::new() && !base_dir.exists() {
        Err(anyhow!("Error: failed to find base_dir \"{}\"", base_dir.display()))
    } else {
        Ok(base_dir)
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn get_list_parameters(plugin_list: &[String], cfg: &Cfg) -> Result<(usize, bool, Mode, PathBuf, bool, bool, bool, bool)> {
    let mut dry_run = cfg.dry_run;
    let mut mode = cfg.mode.clone();
    let mut base_dir = cfg.base_dir.clone();
    let mut no_ignore_errors = cfg.no_ignore_errors;
    let mut strip_masters = cfg.strip_masters;
    let mut reindex = cfg.reindex;
    let mut debug = cfg.debug;
    let mut index = 1;

    loop {
        if plugin_list.len() >= (index + 1) {
            let arg = &plugin_list[index];
            let arg_low = &arg.to_lowercase()[..];
            if arg_low.starts_with("base_dir:") {
                base_dir = get_base_dir(arg).with_context(|| "Failed to get list base_dir")?;
                index += 1;
                continue;
            }
            match arg_low {
                "dry_run" => {
                    dry_run = true;
                    index += 1;
                    continue;
                }
                "no_dry_run" => {
                    dry_run = false;
                    index += 1;
                    continue;
                }
                "keep" => {
                    mode = Mode::Keep;
                    index += 1;
                    continue;
                }
                "keep_without_lands" => {
                    mode = Mode::KeepWithoutLands;
                    index += 1;
                    continue;
                }
                "jobasha" => {
                    mode = Mode::Jobasha;
                    index += 1;
                    continue;
                }
                "jobasha_without_lands" => {
                    mode = Mode::JobashaWithoutLands;
                    index += 1;
                    continue;
                }
                "grass" => {
                    mode = Mode::Grass;
                    index += 1;
                    continue;
                }
                "replace" => {
                    mode = Mode::Replace;
                    index += 1;
                    continue;
                }
                "complete_replace" => {
                    mode = Mode::CompleteReplace;
                    index += 1;
                    continue;
                }
                "ignore_errors" => {
                    no_ignore_errors = false;
                    index += 1;
                    continue;
                }
                "no_ignore_errors" => {
                    no_ignore_errors = true;
                    index += 1;
                    continue;
                }
                "strip_masters" => {
                    strip_masters = true;
                    index += 1;
                    continue;
                }
                "no_strip_masters" => {
                    strip_masters = false;
                    index += 1;
                    continue;
                }
                "reindex" => {
                    reindex = true;
                    index += 1;
                    continue;
                }
                "no_reindex" => {
                    reindex = false;
                    index += 1;
                    continue;
                }
                "debug" => {
                    debug = true;
                    index += 1;
                    continue;
                }
                "no_debug" => {
                    debug = false;
                    index += 1;
                    continue;
                }
                _ => break,
            }
        };
    }
    Ok((index, dry_run, mode, base_dir, no_ignore_errors, strip_masters, reindex, debug))
}

pub(crate) fn references_sorted(references: &mut [&Reference]) {
    references.sort_by_key(|r| {
        (
            std::cmp::Reverse(r.moved_cell),
            r.temporary,
            match r.mast_index {
                0 => u32::MAX,
                i => i,
            },
            r.refr_index,
        )
    });
}

pub(crate) fn process_moved_instances(out: &mut Out, h: &Helper) -> Result<()> {
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
            };
        }
    }
    Ok(())
}

pub(crate) fn show_list_options(
    dry_run: bool,
    mode: &Mode,
    base_dir: &PathBuf,
    no_ignore_errors: bool,
    strip_masters: bool,
    reindex: bool,
    debug: bool,
) -> String {
    let mut text = String::new();
    if dry_run {
        text.push_str("dry_run, ");
    }
    text = format!("{text}mode = {}", mode);
    if base_dir != &PathBuf::new() {
        text = format!("{text}, base_dir = {}", base_dir.display());
    }
    if no_ignore_errors {
        text.push_str(", no_ignore_errors");
    }
    if strip_masters {
        text.push_str(", strip_masters");
    }
    if reindex {
        text.push_str(", reindex");
    }
    if debug {
        text.push_str(", debug");
    }
    text
}

pub(crate) fn show_ignored_ref_errors(
    ignored_ref_errors: &[IgnoredRefError],
    plugin_name_low: &PluginNameLow,
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
            "Warning: probably outdated plugin \"{plugin_name_low}\" contains modified cell reference{ref_suffix} {cell_msg_part}missing from master{master_suffix}:"
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
