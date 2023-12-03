use crate::{
    get_append_to_use_load_order_string, get_base_dir_path, get_game_config_string, get_skip_from_use_load_order_string, msg,
    msg_no_log, show_ignored_ref_errors, truncate_header_text, Bsa, Cfg, Log, Stats, StatsUpdateKind,
};
use anyhow::{anyhow, Context, Result};
use fs_err::read;
use hashbrown::{HashMap, HashSet};
use std::{
    fmt,
    path::PathBuf,
    time::{Instant, SystemTime},
};
use tes3::esp::{
    Activator, Alchemy, Apparatus, Armor, Birthsign, Bodypart, Book, Cell, Class, Clothing, Container, Creature, Dialogue,
    DialogueInfo, Door, EffectId, Enchanting, Faction, GameSetting, GlobalVariable, Ingredient, Landscape, LandscapeTexture,
    LeveledCreature, LeveledItem, Light, Lockpick, MagicEffect, MiscItem, Npc, PathGrid, Plugin, Probe, Race, Reference, Region,
    RepairItem, Script, Skill, SkillId, Sound, SoundGen, Spell, StartScript, Static, Weapon,
};

#[derive(Clone, Default)]
pub(crate) enum Mode {
    #[default]
    Keep,
    KeepWithoutLands,
    Jobasha,
    JobashaWithoutLands,
    Grass,
    Replace,
    CompleteReplace,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Mode::Keep => "keep",
                Mode::KeepWithoutLands => "keep_without_lands",
                Mode::Jobasha => "jobasha",
                Mode::JobashaWithoutLands => "jobasha_without_lands",
                Mode::Grass => "grass",
                Mode::Replace => "replace",
                Mode::CompleteReplace => "complete_replace",
            }
        )?;
        write!(f, "")
    }
}

pub(crate) type CellExtGrid = (i32, i32);
pub(crate) type CellIntNameLow = String;
pub(crate) type GlobalRecordId = usize;
pub(crate) type InfoId = usize;
pub(crate) type InfoName = String;
pub(crate) type MastId = u32;
pub(crate) type LocalVtexId = u16;
pub(crate) type GlobalVtexId = u16;
pub(crate) type PluginName = String;
pub(crate) type MasterNameLow = String;
pub(crate) type PluginNameLow = String;
pub(crate) type RecordNameLow = String;
pub(crate) type RefrId = u32;
pub(crate) type IsExternalRefId = bool;
pub(crate) type IsMovedRefId = bool;
pub(crate) type RefSources = HashMap<(MastId, RefrId), ((MastId, RefrId), IsExternalRefId, IsMovedRefId)>;
pub(crate) type OldRefSources = HashMap<(MastId, RefrId), ((MastId, RefrId), Reference)>;
pub(crate) type FallbackStatics = HashMap<String, (HashMap<RecordNameLow, GlobalRecordId>, Vec<Static>)>;

#[derive(Clone, Default)]
pub(crate) struct ListOptions {
    pub(crate) mode: Mode,
    pub(crate) base_dir: PathBuf,
    pub(crate) dry_run: bool,
    pub(crate) use_load_order: bool,
    pub(crate) config: String,
    pub(crate) show_all_missing_refs: bool,
    pub(crate) turn_normal_grass: bool,
    pub(crate) prefer_loose_over_bsa: bool,
    pub(crate) reindex: bool,
    pub(crate) strip_masters: bool,
    pub(crate) exclude_deleted_records: bool,
    pub(crate) no_show_missing_refs: bool,
    pub(crate) debug: bool,
    pub(crate) no_ignore_errors: bool,
    pub(crate) no_compare: bool,
    pub(crate) no_compare_secondary: bool,
    pub(crate) dry_run_secondary: bool,
    pub(crate) dry_run_dismiss_stats: bool,
    pub(crate) ignore_important_errors: bool,
    pub(crate) regex_case_sensitive: bool,
    pub(crate) regex_sort_by_name: bool,
    pub(crate) insufficient_merge: bool,
    pub(crate) append_to_use_load_order: String,
    pub(crate) skip_from_use_load_order: String,
}

impl ListOptions {
    pub(crate) fn show(&self) -> String {
        let mut text = format!("mode = {}", self.mode);
        if self.base_dir != PathBuf::new() {
            text.push_str(&format!(", base_dir = {}", self.base_dir.display()));
        };
        if self.config != String::new() {
            text.push_str(&format!(", config = {}", self.config));
        };
        if self.append_to_use_load_order != String::new() {
            text.push_str(&format!(", append_to_use_load_order = {}", self.append_to_use_load_order));
        };
        if self.skip_from_use_load_order != String::new() {
            text.push_str(&format!(", skip_from_use_load_order = {}", self.skip_from_use_load_order));
        };
        macro_rules! push_str_if {
            ($($var:ident),+) => {
                $(if self.$var {
                    text.push_str(", ");
                    text.push_str(stringify!($var));
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
            exclude_deleted_records,
            no_show_missing_refs,
            debug,
            no_ignore_errors,
            no_compare,
            no_compare_secondary,
            dry_run_secondary,
            dry_run_dismiss_stats,
            ignore_important_errors,
            regex_case_sensitive,
            regex_sort_by_name,
            insufficient_merge
        );
        text
    }

    pub(crate) fn get_list_options(&self, plugin_list: &[String], cfg: &Cfg, log: &mut Log) -> Result<(usize, ListOptions)> {
        let mut index = 1;
        let mut list_options = self.clone();
        while plugin_list.len() >= (index + 1) {
            let arg = &plugin_list[index];
            let mut arg_low = &arg.to_lowercase().replace('-', "_")[..];
            if let Some(stripped) = arg_low.strip_prefix("__") {
                arg_low = stripped;
            }
            if arg_low.starts_with(&cfg.guts.list_options_prefix_base_dir) {
                list_options.base_dir =
                    get_base_dir_path(arg, cfg).with_context(|| format!("Failed to get list base_dir from \"{}\"", arg))?;
            } else if arg_low.starts_with(&cfg.guts.list_options_prefix_config) {
                list_options.config =
                    get_game_config_string(arg, cfg).with_context(|| format!("Failed to get game config from \"{}\"", arg))?;
            } else if arg_low.starts_with(&cfg.guts.list_options_prefix_append_to_use_load_order) {
                list_options.append_to_use_load_order = get_append_to_use_load_order_string(arg, cfg)
                    .with_context(|| format!("Failed to get plugin path to append to use_load_order from \"{}\"", arg))?;
            } else if arg_low.starts_with(&cfg.guts.list_options_prefix_skip_from_use_load_order) {
                list_options.skip_from_use_load_order = get_skip_from_use_load_order_string(arg, cfg)
                    .with_context(|| format!("Failed to get plugin name to skip from use_load_order from \"{}\"", arg))?;
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
                    "ignore_important_errors" => list_options.ignore_important_errors = true,
                    "no_ignore_important_errors" => list_options.ignore_important_errors = false,
                    "regex_case_sensitive" => list_options.regex_case_sensitive = true,
                    "no_regex_case_sensitive" => list_options.regex_case_sensitive = false,
                    "regex_sort_by_name" => list_options.regex_sort_by_name = true,
                    "no_regex_sort_by_name" => list_options.regex_sort_by_name = false,
                    "insufficient_merge" => list_options.insufficient_merge = true,
                    "no_insufficient_merge" => list_options.insufficient_merge = false,
                    _ => break,
                }
            }
            index += 1;
        }
        mutate_list_options(&mut list_options, cfg, log)?;
        Ok((index, list_options))
    }
}

pub(super) struct RegexPluginInfo {
    pub(super) path: PathBuf,
    pub(super) name_low: String,
    pub(super) time: SystemTime,
}

#[derive(Clone, Default)]
pub(crate) struct PluginInfo {
    #[allow(dead_code)]
    pub(crate) id: usize,
    pub(crate) name: PluginName,
    pub(crate) name_low: PluginNameLow,
    pub(crate) path: PathBuf,
}

pub(crate) struct GlobalMaster {
    pub(crate) global_id: MastId,
    pub(crate) name_low: MasterNameLow,
}

pub(crate) struct LocalMergedMaster {
    pub(crate) local_id: MastId,
    pub(crate) name_low: MasterNameLow,
}

pub(crate) struct LocalMaster {
    pub(crate) local_id: MastId,
    pub(crate) global_id: MastId,
}

pub(crate) struct MergedPluginRefr {
    pub(crate) local_refr: RefrId,
    pub(crate) global_refr: RefrId,
}

pub(crate) struct DialMeta {
    pub(crate) global_dial_id: GlobalRecordId,
    pub(crate) info_metas: HashMap<InfoName, InfoId>,
}

pub(crate) struct CellMeta {
    pub(crate) global_cell_id: GlobalRecordId,
    pub(crate) plugin_metas: Vec<MergedPluginMeta>,
}

pub(crate) struct MergedPluginMeta {
    pub(crate) plugin_name_low: PluginNameLow,
    pub(crate) plugin_refrs: Vec<MergedPluginRefr>,
}

pub(crate) struct Dial {
    pub(crate) dialogue: Dialogue,
    pub(crate) info: Vec<DialogueInfo>,
    pub(crate) excluded_infos: Vec<usize>,
}

pub(crate) struct IgnoredRefError {
    pub(crate) master: MasterNameLow,
    pub(crate) first_encounter: String,
    pub(crate) cell_counter: usize,
    pub(crate) ref_counter: usize,
}

pub(crate) type MovedInstanceId = (MastId, RefrId);

pub(crate) struct MovedInstanceGrids {
    pub(crate) old_grid: CellExtGrid,
    pub(crate) new_grid: CellExtGrid,
}

#[derive(Default)]
pub(crate) struct Helper {
    pub(crate) t: HelperTotal,
    pub(crate) g: HelperGlobal,
    pub(crate) l: HelperLocal,
}

#[derive(Default)]
pub(crate) struct HelperTotal {
    pub(crate) stats: Stats,
    pub(crate) stats_substract_output: Stats,
    pub(crate) stats_tng: Stats,
    pub(crate) game_configs: Vec<GameConfig>,
    pub(crate) assets: Vec<Assets>,
    pub(crate) fallback_statics: Vec<FallbackStatics>,
    pub(crate) skipped_processing_plugins: Vec<String>,
}

#[derive(Default)]
pub(crate) struct GameConfig {
    pub(crate) path: PathBuf,
    pub(crate) path_canonical: PathBuf,
    pub(crate) load_order: LoadOrder,
}

#[derive(Default)]
pub(crate) struct Assets {
    pub(crate) meshes: AssetsType,
    pub(crate) bsa: Vec<Bsa>,
}

#[derive(Default)]
pub(crate) struct AssetsType {
    pub(crate) loose: AssetsLoose,
    pub(crate) bsa: AssetsBsa,
}

#[derive(Default)]
pub(crate) struct AssetsLoose {
    pub(crate) scanned: bool,
    pub(crate) files: HashMap<String, PathBuf>,
}

#[derive(Default, Clone)]
pub(crate) struct FileInBsa {
    pub(crate) path: String,
    pub(crate) bsa_index: usize,
    pub(crate) file_index: usize,
}

#[derive(Default)]
pub(crate) struct AssetsBsa {
    pub(crate) scanned: bool,
    pub(crate) files: HashMap<String, FileInBsa>,
}

#[derive(Default)]
pub(crate) struct LoadOrder {
    pub(crate) scanned: bool,
    pub(crate) contents: Vec<String>,
    pub(crate) groundcovers: Vec<String>,
    pub(crate) datas: Vec<(usize, PathBuf)>,
    pub(crate) fallback_archives: Vec<(usize, String, Option<SystemTime>)>,
}

#[derive(Default)]
pub(crate) struct HelperGlobal {
    pub(crate) list_options: ListOptions,
    pub(crate) plugins_processed: Vec<PluginInfo>,
    pub(crate) masters: Vec<GlobalMaster>,
    pub(crate) refr: RefrId,
    pub(crate) contains_non_external_refs: bool,
    pub(crate) stats: Stats,
    pub(crate) stats_dismiss: bool,
    pub(crate) stats_tng: Stats,
    pub(crate) r: HelperRecords,
    pub(crate) turn_normal_grass: HashMap<String, TurnNormalGrass>,
    pub(crate) found_stat_ids: HashSet<String>,
    pub(crate) config_index: usize,
}

#[derive(Default)]
pub(crate) struct TurnNormalGrass {
    pub(crate) stat_records: Vec<Static>,
    pub(crate) loose: Option<PathBuf>,
    pub(crate) bsa: Option<FileInBsa>,
    pub(crate) new_name_low: String,
    pub(crate) new_path: PathBuf,
    pub(crate) file_contents: Vec<u8>,
    pub(crate) src_info: String,
}

impl TurnNormalGrass {
    pub(crate) fn read_from_bsa(&mut self, bsas: &[Bsa]) -> Result<()> {
        self.file_contents = match &self.bsa {
            None => return Err(anyhow!("Bug: trying to read from BSA, though there is no info about BSA")),
            Some(bsa) => {
                self.src_info = format!("mesh \"{}\" from BSA \"{}\"", bsa.path, bsas[bsa.bsa_index].path);
                bsas[bsa.bsa_index].get_file_by_index(bsa.file_index).with_context(|| {
                    format!(
                        "Failed to get file \"{}\" by index from BSA \"{}\"",
                        bsa.path, bsas[bsa.bsa_index].path
                    )
                })?
            }
        };
        Ok(())
    }

    pub(crate) fn read_from_loose(&mut self) -> Result<()> {
        self.file_contents = match &self.loose {
            None => {
                return Err(anyhow!(
                    "Bug: trying to read from loose file, though there is no info about loose file"
                ))
            }
            Some(path) => match read(path) {
                Ok(file) => {
                    self.src_info = format!("loose mesh \"{}\"", path.display());
                    file
                }
                Err(err) => return Err(anyhow!("Failed to read from file \"{}\", {}", path.display(), err)),
            },
        };
        Ok(())
    }

    pub(crate) fn should_read_from_loose(&self, load_order: &LoadOrder) -> Result<bool> {
        let loose_time = match &self.loose {
            None => {
                return Err(anyhow!(
                    "Bug: trying to get time from loose file, though there is no info about loose file"
                ))
            }
            Some(loose) => match loose.metadata() {
                Err(_) => None,
                Ok(meta) => match meta.modified() {
                    Err(_) => None,
                    Ok(time) => Some(time),
                },
            },
        };
        let bsa_time = match &self.bsa {
            None => return Err(anyhow!("Bug: trying to read time from BSA, though there is no info about BSA")),
            Some(bsa) => match load_order.fallback_archives.get(bsa.bsa_index) {
                None => {
                    return Err(anyhow!(
                        "Bug: trying to get time from BSA, though there is no info about BSA with index \"{}\"",
                        bsa.bsa_index
                    ))
                }
                Some((_, _, time)) => time,
            },
        };
        Ok(loose_time.is_none() || bsa_time.is_none() || loose_time.unwrap() >= bsa_time.unwrap())
    }
}

pub(crate) struct HeaderText {
    pub(crate) author: String,
    pub(crate) description: String,
}

impl HeaderText {
    pub(crate) fn new(author: &str, description: &str, cfg: &Cfg, log: &mut Log) -> Result<HeaderText> {
        let author = truncate_header_text("author", 32, author, cfg, log)?;
        let description = truncate_header_text("description", 256, description, cfg, log)?;
        Ok(HeaderText { author, description })
    }
}

macro_rules! make_helper_records {
    ($($type_simple:ident),+; $($type:ident),+) => {
#[derive(Default)]
pub(crate) struct HelperRecords {
    $(pub(crate) $type_simple: HashMap<RecordNameLow, GlobalRecordId>,)+
    pub(crate) skil: HashMap<SkillId, GlobalRecordId>,
    pub(crate) mgef: HashMap<EffectId, GlobalRecordId>,
    pub(crate) int_cells: HashMap<CellIntNameLow, CellMeta>,
    pub(crate) ext_cells: HashMap<CellExtGrid, CellMeta>,
    pub(crate) ext_ref_sources: HashMap<CellExtGrid, (RefSources, OldRefSources)>,
    pub(crate) moved_instances: HashMap<MovedInstanceId, MovedInstanceGrids>,
    pub(crate) land: HashMap<CellExtGrid, GlobalRecordId>,
    pub(crate) pgrd: HashMap<RecordNameLow, GlobalRecordId>,
    pub(crate) dials: HashMap<RecordNameLow, DialMeta>,
    pub(crate) infos: HashMap<InfoName, RecordNameLow>,
}
        impl HelperRecords {
            pub(crate) fn clear(&mut self) {
                $(self.$type_simple.clear();)+
                $(self.$type.clear();)+
            }
        }
    };
}

make_helper_records!(gmst, glob, clas, fact, race, soun, sndg, scpt, regn, bsgn, sscr, ltex, spel, stat, door, misc, weap, cont, crea, body, ligh, ench, npc_, armo, clot, repa, acti, appa, lock, prob, ingr, book, alch, levi, levc; skil, mgef, int_cells, ext_cells, ext_ref_sources, moved_instances, land, pgrd, dials, infos);

#[derive(Default)]
pub(crate) struct HelperLocal {
    pub(crate) masters: Vec<LocalMaster>,
    pub(crate) merged_masters: Vec<LocalMergedMaster>,
    pub(crate) plugin_info: PluginInfo,
    pub(crate) active_dial_id: Option<GlobalRecordId>,
    pub(crate) active_dial_name_low: RecordNameLow,
    pub(crate) vtex: HashMap<LocalVtexId, GlobalVtexId>,
    pub(crate) ignored_ref_errors: Vec<IgnoredRefError>,
    pub(crate) ignored_cell_errors: Vec<IgnoredRefError>,
    pub(crate) stats: Stats,
}

impl Helper {
    pub(crate) fn new() -> Helper {
        let mut helper = Helper::default();
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
        self.g.stats.add_merged_plugin();
        self.g.stats.add(&self.l.stats);
        if !self.g.list_options.no_show_missing_refs {
            show_ignored_ref_errors(&self.l.ignored_cell_errors, &self.l.plugin_info.name, true, cfg, log)?;
            show_ignored_ref_errors(&self.l.ignored_ref_errors, &self.l.plugin_info.name, false, cfg, log)?;
        }
        self.g.plugins_processed.push(self.l.plugin_info.clone());
        Ok(())
    }

    pub(crate) fn global_commit(&mut self, timer: Instant, new_plugin: &mut Plugin, cfg: &Cfg, log: &mut Log) -> Result<()> {
        self.g.stats.add_result_plugin();
        if !self.g.stats.self_check() {
            return Err(anyhow!("Error(possible bug): record counts self-check for the list failed"));
        }
        if !self.g.stats_dismiss {
            self.g.stats.tes3(StatsUpdateKind::ResultUnique);
            self.g.stats.header_adjust();
        } else {
            self.t.stats_substract_output.add_output(&self.g.stats);
        }
        self.t.stats.add(&self.g.stats);
        if self.g.stats_dismiss {
            self.g.stats.reset_output();
        }
        self.t.stats_tng.add(&self.g.stats_tng);
        self.g.stats.add(&self.g.stats_tng);
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
        if !self.t.stats.self_check() {
            return Err(anyhow!("Error(possible bug): total record counts self-check failed"));
        }
        self.t.stats.substract(&self.t.stats_substract_output);
        self.t.stats.add(&self.t.stats_tng);
        let mut text = format!("{}\n{}", cfg.guts.prefix_combined_stats, self.t.stats.total_string(timer));
        if cfg.verbose < 2 {
            msg_no_log(&text, 0, cfg);
        }
        text = format!("{}{}", text, self.t.stats);
        msg(text, 2, cfg, log)?;
        if !self.t.skipped_processing_plugins.is_empty() && cfg.verbose < cfg.guts.skipped_processing_plugins_msg_verbosity {
            let skipped_processing_plugins_len = self.t.skipped_processing_plugins.len();
            let text = format!(
                "Skipped processing {} plugin{}({}add -{} to get list)",
                skipped_processing_plugins_len,
                if skipped_processing_plugins_len == 1 { "" } else { "s" },
                if cfg.no_log { "" } else { "check log or " },
                "v".repeat(cfg.guts.skipped_processing_plugins_msg_verbosity as usize),
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

fn get_plugin_info(path: PathBuf, id: usize) -> Result<PluginInfo> {
    match path.file_name() {
        Some(file_name) => {
            let name = file_name.to_string_lossy().into_owned();
            let name_low = name.to_lowercase();
            Ok(PluginInfo { id, name, name_low, path })
        }
        None => Err(anyhow!("Failed to get plugin name for \"{}\"", path.display())),
    }
}

fn mutate_list_options(list_options: &mut ListOptions, cfg: &Cfg, log: &mut Log) -> Result<()> {
    let mut text = Vec::new();
    if list_options.exclude_deleted_records && !list_options.use_load_order {
        text.push("List options mutation: Implicitly set \"use_load_order\" due to \"exclude_deleted_records\"");
        list_options.use_load_order = true;
    }
    if list_options.use_load_order && list_options.base_dir != PathBuf::new() {
        text.push("List options mutation: Implicitly set \"base_dir:\"(empty) due to \"use_load_order\"");
        list_options.base_dir = PathBuf::new();
    }
    if matches!(list_options.mode, Mode::Grass) && list_options.turn_normal_grass {
        text.push("List options mutation: Implicitly set \"no_turn_normal_grass\" due to \"grass\" mode");
        list_options.turn_normal_grass = false;
    }
    if matches!(list_options.mode, Mode::Grass) && !list_options.insufficient_merge {
        text.push("List options mutation: Implicitly set \"insufficient_merge\" due to \"grass\" mode");
        list_options.insufficient_merge = true;
    }
    if !text.is_empty() {
        msg(text.join("\n"), 1, cfg, log)?;
    }
    Ok(())
}

macro_rules! make_out {
    ($($type:ident, $obj:ident);+) => {
        #[derive(Default)]
        pub(crate) struct Out {
            pub(crate) masters: Vec<(String, u64)>,
            $(pub(crate) $type: Vec<($obj, Vec<$obj>)>,)+
        }
    };
}

make_out!(gmst, GameSetting; glob, GlobalVariable; clas, Class; fact, Faction; race, Race; soun, Sound; sndg, SoundGen; skil, Skill; mgef, MagicEffect; scpt, Script; regn, Region; bsgn, Birthsign; sscr, StartScript; ltex, LandscapeTexture; spel, Spell; stat, Static; door, Door; misc, MiscItem; weap, Weapon; cont, Container; crea, Creature; body, Bodypart; ligh, Light; ench, Enchanting; npc_, Npc; armo, Armor; clot, Clothing; repa, RepairItem; acti, Activator; appa, Apparatus; lock, Lockpick; prob, Probe; ingr, Ingredient; book, Book; alch, Alchemy; levi, LeveledItem; levc, LeveledCreature; cell, Cell; land, Landscape; pgrd, PathGrid; dial, Dial);
