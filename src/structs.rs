use crate::{msg, msg_no_log, show_ignored_ref_errors, Cfg, Log, Stats, StatsUpdateKind};
use anyhow::{anyhow, Result};
use hashbrown::HashMap;
use std::{
    fmt,
    path::{Path, PathBuf},
    time::Instant,
};
use tes3::esp::{
    Activator, Alchemy, Apparatus, Armor, Birthsign, Bodypart, Book, Cell, Class, Clothing, Container, Creature, Dialogue,
    DialogueInfo, Door, EffectId, Enchanting, Faction, GameSetting, GlobalVariable, Ingredient, Landscape, LandscapeTexture,
    LeveledCreature, LeveledItem, Light, Lockpick, MagicEffect, MiscItem, Npc, PathGrid, Plugin, Probe, Race, Region, RepairItem,
    Script, Skill, SkillId, Sound, SoundGen, Spell, StartScript, Static, Weapon,
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
pub(crate) type MasterNameLow = String;
pub(crate) type PluginNameLow = String;
pub(crate) type RecordNameLow = String;
pub(crate) type RefrId = u32;

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
}

#[derive(Default)]
pub(crate) struct HelperGlobal {
    pub(crate) plugins_processed: Vec<PluginNameLow>,
    pub(crate) masters: Vec<GlobalMaster>,
    pub(crate) refr: RefrId,
    pub(crate) dry_run: bool,
    pub(crate) mode: Mode,
    pub(crate) base_dir: PathBuf,
    pub(crate) no_ignore_errors: bool,
    pub(crate) strip_masters: bool,
    pub(crate) reindex: bool,
    pub(crate) debug: bool,
    pub(crate) contains_non_external_refs: bool,
    pub(crate) stats: Stats,
    pub(crate) r: HelperRecords,
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
    pub(crate) moved_instances: HashMap<MovedInstanceId, MovedInstanceGrids>,
    pub(crate) land: HashMap<CellExtGrid, GlobalRecordId>,
    pub(crate) pgrd: HashMap<RecordNameLow, GlobalRecordId>,
    pub(crate) dials: HashMap<RecordNameLow, DialMeta>,
}
        impl HelperRecords {
            pub(crate) fn clear(&mut self) {
                $(self.$type_simple.clear();)+
                $(self.$type.clear();)+
            }
        }
    };
}

make_helper_records!(gmst, glob, clas, fact, race, soun, sndg, scpt, regn, bsgn, sscr, ltex, spel, stat, door, misc, weap, cont, crea, body, ligh, ench, npc_, armo, clot, repa, acti, appa, lock, prob, ingr, book, alch, levi, levc; skil, mgef, int_cells, ext_cells, moved_instances, land, pgrd, dials);

#[derive(Default)]
pub(crate) struct HelperLocal {
    pub(crate) masters: Vec<LocalMaster>,
    pub(crate) merged_masters: Vec<LocalMergedMaster>,
    pub(crate) plugin_name_low: PluginNameLow,
    pub(crate) active_dial_id: Option<GlobalRecordId>,
    pub(crate) active_dial_name_low: RecordNameLow,
    pub(crate) vtex: HashMap<LocalVtexId, GlobalVtexId>,
    pub(crate) ignored_ref_errors: Vec<IgnoredRefError>,
    pub(crate) ignored_cell_errors: Vec<IgnoredRefError>,
    pub(crate) stats: Stats,
}

impl Helper {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn global_init(
        &mut self,
        dry_run: bool,
        mode: Mode,
        base_dir: PathBuf,
        no_ignore_errors: bool,
        strip_masters: bool,
        reindex: bool,
        debug: bool,
    ) {
        self.g.plugins_processed.clear();
        self.g.masters.clear();
        self.g.refr = 0;
        self.g.dry_run = dry_run;
        self.g.mode = mode;
        self.g.base_dir = base_dir;
        self.g.no_ignore_errors = no_ignore_errors;
        self.g.strip_masters = strip_masters;
        self.g.reindex = reindex;
        self.g.debug = debug;
        self.g.contains_non_external_refs = false;
        self.g.stats.reset();
        self.g.r.clear();
    }

    pub(crate) fn local_init(&mut self, plugin_name: &str) -> Result<()> {
        self.l.masters.clear();
        self.l.merged_masters.clear();
        self.l.plugin_name_low = get_plugin_name_low(plugin_name)?;
        self.l.active_dial_id = None;
        self.l.vtex.clear();
        self.l.ignored_cell_errors.clear();
        self.l.ignored_ref_errors.clear();
        self.l.stats.reset();
        Ok(())
    }

    pub(crate) fn local_commit(&mut self, cfg: &Cfg, log: &mut Log) -> Result<()> {
        self.g.plugins_processed.push(self.l.plugin_name_low.clone());
        self.g.stats.add_merged_plugin();
        self.g.stats.add(&self.l.stats);
        show_ignored_ref_errors(&self.l.ignored_cell_errors, &self.l.plugin_name_low, true, cfg, log)?;
        show_ignored_ref_errors(&self.l.ignored_ref_errors, &self.l.plugin_name_low, false, cfg, log)?;
        Ok(())
    }

    pub(crate) fn global_commit(
        &mut self,
        timer: Instant,
        new_plugin: &mut Plugin,
        old_plugin: &mut Plugin,
        cfg: &Cfg,
        log: &mut Log,
    ) -> Result<()> {
        self.g.stats.add_result_plugin();
        if !self.g.stats.self_check() {
            return Err(anyhow!("Error(possible bug): record counts self-check for the list failed"));
        }
        self.g.stats.tes3(StatsUpdateKind::ResultUnique);
        self.g.stats.header_adjust();
        self.t.stats.add(&self.g.stats);
        let mut text = self.g.stats.total_string(timer);
        if cfg.verbose >= 1 && cfg.verbose < 3 {
            msg_no_log(&text, 1, cfg);
        }
        text = format!("{}{}", text, self.g.stats);
        msg(text, 3, cfg, log)?;
        new_plugin.objects.clear();
        old_plugin.objects.clear();
        Ok(())
    }

    pub(crate) fn total_commit(&mut self, timer: Instant, cfg: &Cfg, log: &mut Log) -> Result<()> {
        if !self.t.stats.self_check() {
            return Err(anyhow!("Error(possible bug): total record counts self-check failed"));
        }
        let mut text = format!("{}\n{}", cfg.guts.prefix_combined_stats, self.t.stats.total_string(timer));
        if cfg.verbose < 2 {
            msg_no_log(&text, 0, cfg);
        }
        text = format!("{}{}", text, self.t.stats);
        msg(text, 2, cfg, log)?;
        Ok(())
    }
}

fn get_plugin_name_low(plugin_name: &str) -> Result<String> {
    match Path::new(&plugin_name).file_name() {
        Some(file_name) => Ok(file_name.to_string_lossy().to_lowercase()),
        None => Err(anyhow!("Failed to get plugin name for \"{}\"", plugin_name)),
    }
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
