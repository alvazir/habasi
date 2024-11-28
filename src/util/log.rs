use super::{create_dir_early, msg};
use crate::Cfg;
use anyhow::{anyhow, Context as _, Result};
use fs_err::{rename, File};
use std::{
    io::{self, BufWriter, Write as _},
    path::PathBuf,
};

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
