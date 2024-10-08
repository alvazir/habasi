use super::Log;
use crate::Cfg;
use anyhow::{anyhow, Context, Result};

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
