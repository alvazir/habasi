use super::{
    get_plugin_info, Assets, FallbackStatics, GameConfig, HelperGlobal, HelperLocal, HelperTotal,
    ListOptions,
};
use crate::{msg, msg_no_log, show_ignored_ref_errors, Cfg, Log, StatsUpdateKind};
use anyhow::{anyhow, Context, Result};
use std::{path::PathBuf, time::Instant};
use tes3::esp::Plugin;

#[derive(Default)]
pub struct Helper {
    pub(crate) t: HelperTotal,
    pub(crate) g: HelperGlobal,
    pub(crate) l: HelperLocal,
}

impl Helper {
    pub(crate) fn new() -> Self {
        let mut helper = Self::default();
        helper.t.missing_ref_text.reserve(256);
        helper.g.config_index = usize::MAX;
        helper
    }

    pub(crate) fn global_init(&mut self, list_options: ListOptions) {
        self.g.list_options = list_options;
        self.g.plugins_processed.clear();
        self.g.masters.clear();
        self.g.refr = 0;
        self.g.contains_non_external_refs = false;
        self.g.stats.reset();
        self.g.stats_dismiss = false;
        self.g.stats_tng.reset();
        self.g.r.clear();
        self.g.turn_normal_grass.clear();
        self.g.found_stat_ids.clear();
        self.g.config_index = usize::MAX;
    }

    pub(crate) fn local_init(&mut self, plugin_path: PathBuf, plugin_id: usize) -> Result<()> {
        self.l.masters.clear();
        self.l.merged_masters.clear();
        self.l.plugin_info = get_plugin_info(plugin_path, plugin_id)?;
        self.l.active_dial_id = None;
        self.l.vtex.clear();
        self.l.ignored_cell_errors.clear();
        self.l.ignored_ref_errors.clear();
        self.l.stats.reset();
        Ok(())
    }

    pub(crate) fn local_commit(&mut self, cfg: &Cfg, log: &mut Log) -> Result<()> {
        self.g.stats.add_merged_plugin()?;
        self.g.stats.add(&self.l.stats)?;
        if !self.g.list_options.no_show_missing_refs {
            show_ignored_ref_errors(
                &self.l.ignored_cell_errors,
                &self.l.plugin_info.name,
                true,
                cfg,
                log,
            )?;
            show_ignored_ref_errors(
                &self.l.ignored_ref_errors,
                &self.l.plugin_info.name,
                false,
                cfg,
                log,
            )?;
        }
        self.g.plugins_processed.push(self.l.plugin_info.clone());
        Ok(())
    }

    pub(crate) fn global_commit(
        &mut self,
        timer: Instant,
        new_plugin: &mut Plugin,
        cfg: &Cfg,
        log: &mut Log,
    ) -> Result<()> {
        self.g.stats.add_result_plugin()?;
        if !self.g.stats.self_check()? {
            return Err(anyhow!(
                "Error(possible bug): record counts self-check for the list failed"
            ));
        }
        if self.g.stats_dismiss {
            self.t.stats_substract_output.add_output(&self.g.stats)?;
        } else {
            self.g.stats.tes3(StatsUpdateKind::ResultUnique);
            self.g.stats.header_adjust()?;
        }
        self.t.stats.add(&self.g.stats)?;
        if self.g.stats_dismiss {
            self.g.stats.reset_output();
        }
        self.t.stats_tng.add(&self.g.stats_tng)?;
        self.g.stats.add(&self.g.stats_tng)?;
        let mut text = self.g.stats.total_string(timer);
        if cfg.verbose >= 1 && cfg.verbose < 3 {
            msg_no_log(&text, 1, cfg);
        }
        text = format!("{}{}", text, self.g.stats);
        msg(text, 3, cfg, log)?;
        new_plugin.objects.clear();
        Ok(())
    }

    pub(crate) fn total_commit(&mut self, timer: Instant, cfg: &Cfg, log: &mut Log) -> Result<()> {
        if !self.t.stats.self_check().with_context(|| "")? {
            return Err(anyhow!(
                "Error(possible bug): total record counts self-check failed"
            ));
        }
        self.t.stats.substract(&self.t.stats_substract_output)?;
        self.t.stats.add(&self.t.stats_tng)?;
        let mut text = format!(
            "{}\n{}",
            cfg.guts.prefix_combined_stats,
            self.t.stats.total_string(timer)
        );
        if cfg.verbose < 2 {
            msg_no_log(&text, 0, cfg);
        }
        text = format!("{}{}", text, self.t.stats);
        msg(text, 2, cfg, log)?;
        if !self.t.skipped_processing_plugins.is_empty()
            && cfg.verbose < cfg.guts.skipped_processing_plugins_msg_verbosity
        {
            let skipped_processing_plugins_len = self.t.skipped_processing_plugins.len();
            text = format!(
                "Skipped processing {} plugin{}({}add -{} to get list)",
                skipped_processing_plugins_len,
                if skipped_processing_plugins_len == 1 {
                    ""
                } else {
                    "s"
                },
                if cfg.no_log { "" } else { "check log or " },
                "v".repeat(usize::from(
                    cfg.guts.skipped_processing_plugins_msg_verbosity
                )),
            );
            msg(text, 0, cfg, log)?;
        }
        Ok(())
    }

    pub(crate) fn total_add_skipped_processing_plugin(&mut self, msg: String) {
        if !self.t.skipped_processing_plugins.contains(&msg) {
            self.t.skipped_processing_plugins.push(msg);
        }
    }

    pub(crate) fn add_game_config(&mut self, path: PathBuf, path_canonical: PathBuf) {
        self.t.game_configs.push(GameConfig {
            path,
            path_canonical,
            ..Default::default()
        });
        self.t.assets.push(Assets::default());
        self.t.fallback_statics.push(FallbackStatics::new());
    }
}
