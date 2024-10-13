use super::{msg, prepare_complex_arg_string, Log};
use crate::{load_order::scan, Cfg, Helper, ListOptions, Mode};
use anyhow::{Context, Result};

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
        scan(h, cfg, log).with_context(|| "Failed to scan load order")?;
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

pub fn get_skip_plugin_name_low(h: &Helper) -> String {
    if h.g.list_options.use_load_order && !h.g.list_options.skip_from_use_load_order.is_empty() {
        h.g.list_options.skip_from_use_load_order.to_lowercase()
    } else {
        String::new()
    }
}
