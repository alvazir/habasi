use super::Mode;
use crate::{
    get_append_to_use_load_order_string, get_base_dir_path, get_game_config_string,
    get_skip_from_use_load_order_string, msg, increment, Cfg, Log
};
use anyhow::{Context as _, Result};
use std::{
    fmt::Write as _,
    path::PathBuf,
};

#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Default)]
pub struct IndirectListOptions {
    pub(crate) base_dir: PathBuf,
    pub(crate) base_dir_load_order: PathBuf,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Default)]
pub struct ListOptions {
    pub(crate) mode: Mode,
    pub(crate) base_dir_indirect: PathBuf,
    pub(crate) dry_run: bool,
    pub(crate) use_load_order: bool,
    pub(crate) config: String,
    pub(crate) show_all_missing_refs: bool,
    pub(crate) turn_normal_grass: bool,
    pub(crate) prefer_loose_over_bsa: bool,
    pub(crate) reindex: bool,
    pub(crate) strip_masters: bool,
    pub(crate) force_base_dir: bool,
    pub(crate) exclude_deleted_records: bool,
    pub(crate) no_show_missing_refs: bool,
    pub(crate) debug: bool,
    pub(crate) no_ignore_errors: bool,
    pub(crate) no_compare: bool,
    pub(crate) no_compare_secondary: bool,
    pub(crate) dry_run_secondary: bool,
    pub(crate) dry_run_dismiss_stats: bool,
    pub(crate) regex_case_sensitive: bool,
    pub(crate) regex_sort_by_name: bool,
    pub(crate) force_dial_type: bool,
    pub(crate) ignore_important_errors: bool,
    pub(crate) insufficient_merge: bool,
    pub(crate) append_to_use_load_order: String,
    pub(crate) skip_from_use_load_order: String,
    pub(crate) indirect: IndirectListOptions,
}

impl ListOptions {
    #[allow(clippy::cognitive_complexity)]
    pub(crate) fn show(&self) -> Result<String> {
        let mut text = format!("mode = {}", self.mode);
        if !self.base_dir_indirect.as_os_str().is_empty() {
            write!(
                text,
                ", base_dir = \"{}\"",
                self.base_dir_indirect.display()
            )?;
        };
        if !self.config.is_empty() {
            write!(text, ", config = \"{}\"", self.config)?;
        };
        if !self.append_to_use_load_order.is_empty() {
            write!(
                text,
                ", append_to_use_load_order = \"{}\"",
                self.append_to_use_load_order
            )?;
        };
        if !self.skip_from_use_load_order.is_empty() {
            write!(
                text,
                ", skip_from_use_load_order = \"{}\"",
                self.skip_from_use_load_order
            )?;
        };
        macro_rules! push_str_if {
            ($($var:ident),+) => {
                $(if self.$var {
                    write!(text, ", {}", stringify!($var))?;
                })+
            };
        }
        push_str_if!(
            dry_run,
            use_load_order,
            show_all_missing_refs,
            turn_normal_grass,
            prefer_loose_over_bsa,
            reindex,
            strip_masters,
            force_base_dir,
            exclude_deleted_records,
            no_show_missing_refs,
            debug,
            no_ignore_errors,
            no_compare,
            no_compare_secondary,
            dry_run_secondary,
            dry_run_dismiss_stats,
            regex_case_sensitive,
            regex_sort_by_name,
            force_dial_type,
            ignore_important_errors,
            insufficient_merge
        );
        Ok(text)
    }

    // COMMENT: used for passing config path, ignore_errors, base_dir to scan in use_load_order/preset
    pub(crate) fn get_pristine(&self) -> Self {
        self.clone()
    }

    pub(crate) fn get_mutated(
        &self,
        plugin_list: &[String],
        cfg: &Cfg,
        log: &mut Log,
    ) -> Result<(usize, Self)> {
        let mut index: usize = 1;
        let mut list_options = self.clone();
        while plugin_list.len()
            >= increment!(index)
        {
            let arg = &plugin_list
                .get(index)
                .with_context(|| format!("Bug: indexing slicing plugin_list[{index}]"))?;
            let mut arg_low = &*arg.to_lowercase().replace('-', "_");
            if let Some(stripped) = arg_low.strip_prefix("__") {
                arg_low = stripped;
            }
            if arg_low.starts_with(&cfg.guts.list_options_prefix_base_dir) {
                list_options.base_dir_indirect = get_base_dir_path(arg, cfg)
                    .with_context(|| format!("Failed to get list base_dir from {arg:?}"))?;
            } else if arg_low.starts_with(&cfg.guts.list_options_prefix_config) {
                list_options.config = get_game_config_string(arg, cfg)
                    .with_context(|| format!("Failed to get game config from {arg:?}"))?;
            } else if arg_low.starts_with(&cfg.guts.list_options_prefix_append_to_use_load_order) {
                list_options.append_to_use_load_order =
                    get_append_to_use_load_order_string(arg, cfg).with_context(|| {
                        format!(
                            "Failed to get plugin path to append to use_load_order from {arg:?}"
                        )
                    })?;
            } else if arg_low.starts_with(&cfg.guts.list_options_prefix_skip_from_use_load_order) {
                list_options.skip_from_use_load_order =
                    get_skip_from_use_load_order_string(arg, cfg).with_context(|| {
                        format!(
                            "Failed to get plugin name to skip from use_load_order from {arg:?}"
                        )
                    })?;
            } else {
                match arg_low {
                    "keep" => list_options.mode = Mode::Keep,
                    "keep_without_lands" => list_options.mode = Mode::KeepWithoutLands,
                    "jobasha" => list_options.mode = Mode::Jobasha,
                    "jobasha_without_lands" => list_options.mode = Mode::JobashaWithoutLands,
                    "replace" => list_options.mode = Mode::Replace,
                    "complete_replace" => list_options.mode = Mode::CompleteReplace,
                    "grass" => list_options.mode = Mode::Grass,
                    "dry_run" => list_options.dry_run = true,
                    "no_dry_run" => list_options.dry_run = false,
                    "use_load_order" => list_options.use_load_order = true,
                    "no_use_load_order" => list_options.use_load_order = false,
                    "show_all_missing_refs" => list_options.show_all_missing_refs = true,
                    "no_show_all_missing_refs" => list_options.show_all_missing_refs = false,
                    "turn_normal_grass" => list_options.turn_normal_grass = true,
                    "no_turn_normal_grass" => list_options.turn_normal_grass = false,
                    "prefer_loose_over_bsa" => list_options.prefer_loose_over_bsa = true,
                    "no_prefer_loose_over_bsa" => list_options.prefer_loose_over_bsa = false,
                    "reindex" => list_options.reindex = true,
                    "no_reindex" => list_options.reindex = false,
                    "strip_masters" => list_options.strip_masters = true,
                    "no_strip_masters" => list_options.strip_masters = false,
                    "force_base_dir" => list_options.force_base_dir = true,
                    "no_force_base_dir" => list_options.force_base_dir = false,
                    "exclude_deleted_records" => list_options.exclude_deleted_records = true,
                    "no_exclude_deleted_records" => list_options.exclude_deleted_records = false,
                    "no_show_missing_refs" => list_options.no_show_missing_refs = true,
                    "show_missing_refs" => list_options.no_show_missing_refs = false,
                    "debug" => list_options.debug = true,
                    "no_debug" => list_options.debug = false,
                    "ignore_errors" => list_options.no_ignore_errors = false,
                    "no_ignore_errors" => list_options.no_ignore_errors = true,
                    "no_compare" => list_options.no_compare = true,
                    "compare" => list_options.no_compare = false,
                    "no_compare_secondary" => list_options.no_compare_secondary = true,
                    "compare_secondary" => list_options.no_compare_secondary = false,
                    "dry_run_secondary" => list_options.dry_run_secondary = true,
                    "no_dry_run_secondary" => list_options.dry_run_secondary = false,
                    "dry_run_dismiss_stats" => list_options.dry_run_dismiss_stats = true,
                    "no_dry_run_dismiss_stats" => list_options.dry_run_dismiss_stats = false,
                    "regex_case_sensitive" => list_options.regex_case_sensitive = true,
                    "no_regex_case_sensitive" => list_options.regex_case_sensitive = false,
                    "regex_sort_by_name" => list_options.regex_sort_by_name = true,
                    "no_regex_sort_by_name" => list_options.regex_sort_by_name = false,
                    "force_dial_type" => list_options.force_dial_type = true,
                    "no_force_dial_type" => list_options.force_dial_type = false,
                    "ignore_important_errors" => list_options.ignore_important_errors = true,
                    "no_ignore_important_errors" => list_options.ignore_important_errors = false,
                    "insufficient_merge" => list_options.insufficient_merge = true,
                    "no_insufficient_merge" => list_options.insufficient_merge = false,
                    _ => break,
                }
            }
            index = increment!(index);
        }
        list_options.mutate(cfg, log)?;
        Ok((index, list_options))
    }

    fn mutate(&mut self, cfg: &Cfg, log: &mut Log) -> Result<()> {
        let mut text = String::new();
        let prefix = "List options: Implicitly";
        if self.exclude_deleted_records && !self.use_load_order {
            writeln!(&mut text, "{prefix} set \"use_load_order\" due to \"exclude_deleted_records\"")?;
            self.use_load_order = true;
        }
        if self.force_base_dir && !self.use_load_order {
            writeln!(&mut text, "{prefix} unset \"force_base_dir\" due to lack of \"use_load_order\"")?;
            self.force_base_dir = false;
        }
        if !self.base_dir_indirect.as_os_str().is_empty() {
            if self.use_load_order {
                if self.force_base_dir {
                    self.indirect.base_dir_load_order = self.base_dir_indirect.clone();
                } else {
                    writeln!(&mut text, 
                    "{prefix} set \"base_dir:\"(empty) due to \"use_load_order\" and lack of \"force_base_dir\"",
                )?;
                    self.base_dir_indirect = PathBuf::new();
                }
            } else {
                self.indirect.base_dir = self.base_dir_indirect.clone();
            }
        }
        if self.force_dial_type && self.insufficient_merge {
            writeln!(&mut text, "{prefix} unset \"force_dial_type\" due to \"insufficient_merge\"")?;
            self.force_dial_type = false;
        }
        if matches!(self.mode, Mode::Grass) {
            if self.turn_normal_grass {
                writeln!(&mut text, "{prefix} unset \"turn_normal_grass\" due to \"grass\" mode")?;
                self.turn_normal_grass = false;
            };
            if !self.insufficient_merge {
                writeln!(&mut text, "{prefix} set \"insufficient_merge\" due to \"grass\" mode")?;
                self.insufficient_merge = true;
            }
        }
        if !text.is_empty() {
            msg(text, 1, cfg, log)?;
        }
        Ok(())
    }
}

