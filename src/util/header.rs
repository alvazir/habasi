use super::{msg, msg_no_log, Log};
use crate::{Cfg, Helper};
use anyhow::{Context, Result};
use std::fmt::Write as _;

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
