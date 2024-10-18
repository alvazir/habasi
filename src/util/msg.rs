use super::Log;
use crate::Cfg;
use anyhow::{anyhow, Context, Result};

const PREFIX_IGNORED_IMPORTANT_ERROR: &str = "Ignored important error: ";
const SUGGESTION_IGNORE_IMPORTANT_ERRORS: &str = "\n\tFix the problem or add \"--ignore-important-errors\"(may rarely cause unexpected behaviour) to ignore";

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

#[allow(clippy::module_name_repetitions)]
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
    let body = text.as_ref();
    if ignore {
        let message = format!("{PREFIX_IGNORED_IMPORTANT_ERROR}{body}");
        msg(message, 0, cfg, log)
    } else {
        Err(anyhow!(format!(
            "{body}{tag}{SUGGESTION_IGNORE_IMPORTANT_ERRORS}",
            tag = if unexpected_tag {
                "\n\tConsider reporting the error to add this tag to the list of unexpected tags to skip by default"
            } else {
                ""
            }
        )))
    }
}

pub fn err_or_ignore_thread_safe<S: AsRef<str>>(text: S, ignore: bool, cfg: &Cfg) -> Result<()> {
    let body = text.as_ref();
    if ignore {
        msg_no_log(format!("{PREFIX_IGNORED_IMPORTANT_ERROR}{body}"), 0, cfg);
        Ok(())
    } else {
        Err(anyhow!(format!(
            "{body}{SUGGESTION_IGNORE_IMPORTANT_ERRORS}"
        )))
    }
}
