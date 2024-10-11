use crate::{truncate_header_text, Bsa, Cfg, Log, Stats};
use anyhow::{anyhow, Result};
use hashbrown::{HashMap, HashSet};
use std::{path::PathBuf, time::SystemTime};
use tes3::esp::{EffectId, Reference, SkillId, Static};
pub mod dial;
pub mod helper;
pub mod list_options;
pub mod mode;
pub mod out;
pub mod turn_normal_grass;
use dial::{Dial, DialMeta};
use list_options::ListOptions;
use mode::Mode;
use turn_normal_grass::TurnNormalGrass;

pub type CellExtGrid = (i32, i32);
pub type CellIntNameLow = String;
pub type GlobalRecordId = usize;
pub type InfoId = usize;
pub type InfoName = String;
pub type MastId = u32;
pub type LocalVtexId = u16;
pub type GlobalVtexId = u16;
pub type PluginName = String;
pub type MasterNameLow = String;
pub type PluginNameLow = String;
pub type RecordNameLow = String;
pub type RefrId = u32;
pub type IsExternalRefId = bool;
pub type IsMovedRefId = bool;
pub type RefSources = HashMap<(MastId, RefrId), ((MastId, RefrId), IsExternalRefId, IsMovedRefId)>;
pub type OldRefSources = HashMap<(MastId, RefrId), ((MastId, RefrId), Reference)>;
pub type FallbackStatics = HashMap<String, (HashMap<RecordNameLow, GlobalRecordId>, Vec<Static>)>;
pub type MovedInstanceId = (MastId, RefrId);

#[derive(Default)]
pub struct HelperTotal {
    pub(crate) stats: Stats,
    pub(crate) stats_substract_output: Stats,
    pub(crate) stats_tng: Stats,
    pub(crate) game_configs: Vec<GameConfig>,
    pub(crate) assets: Vec<Assets>,
    pub(crate) fallback_statics: Vec<FallbackStatics>,
    pub(crate) skipped_processing_plugins: Vec<String>,
    pub(crate) missing_ref_text: String,
}

#[derive(Default)]
pub struct HelperGlobal {
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
pub struct HelperLocal {
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

pub struct RegexPluginInfo {
    pub(super) path: PathBuf,
    pub(super) name_low: String,
    pub(super) time: SystemTime,
}

#[derive(Clone, Default)]
pub struct PluginInfo {
    #[allow(dead_code)]
    pub(crate) id: usize,
    pub(crate) name: PluginName,
    pub(crate) name_low: PluginNameLow,
    pub(crate) path: PathBuf,
}

pub struct GlobalMaster {
    pub(crate) global_id: MastId,
    pub(crate) name_low: MasterNameLow,
}

pub struct LocalMergedMaster {
    pub(crate) local_id: MastId,
    pub(crate) name_low: MasterNameLow,
}

pub struct LocalMaster {
    pub(crate) local_id: MastId,
    pub(crate) global_id: MastId,
}

pub struct MergedPluginRefr {
    pub(crate) local_refr: RefrId,
    pub(crate) global_refr: RefrId,
}

pub struct CellMeta {
    pub(crate) global_cell_id: GlobalRecordId,
    pub(crate) plugin_metas: Vec<MergedPluginMeta>,
}

pub struct MergedPluginMeta {
    pub(crate) plugin_name_low: PluginNameLow,
    pub(crate) plugin_refrs: Vec<MergedPluginRefr>,
}

pub struct IgnoredRefError {
    pub(crate) master: MasterNameLow,
    pub(crate) first_encounter: String,
    pub(crate) cell_counter: usize,
    pub(crate) ref_counter: usize,
}

pub struct MovedInstanceGrids {
    pub(crate) old_grid: CellExtGrid,
    pub(crate) new_grid: CellExtGrid,
}

#[derive(Default)]
pub struct GameConfig {
    pub(crate) path: PathBuf,
    pub(crate) path_canonical: PathBuf,
    pub(crate) load_order: LoadOrder,
}

#[derive(Default)]
pub struct Assets {
    pub(crate) meshes: AssetsType,
    pub(crate) bsa: Vec<Bsa>,
}

#[derive(Default)]
pub struct AssetsType {
    pub(crate) loose: AssetsLoose,
    pub(crate) bsa: AssetsBsa,
}

#[derive(Default)]
pub struct AssetsLoose {
    pub(crate) scanned: bool,
    pub(crate) files: HashMap<String, PathBuf>,
}

#[derive(Default, Clone)]
pub struct FileInBsa {
    pub(crate) path: String,
    pub(crate) bsa_index: usize,
    pub(crate) file_index: usize,
}

#[derive(Default)]
pub struct AssetsBsa {
    pub(crate) scanned: bool,
    pub(crate) files: HashMap<String, FileInBsa>,
}

#[derive(Default)]
pub struct LoadOrder {
    pub(crate) scanned: bool,
    pub(crate) contents: Vec<String>,
    pub(crate) groundcovers: Vec<String>,
    pub(crate) datas: Vec<(usize, PathBuf)>,
    pub(crate) fallback_archives: Vec<(usize, String, Option<SystemTime>)>,
}

pub struct HeaderText {
    pub(crate) author: String,
    pub(crate) description: String,
}

impl HeaderText {
    pub(crate) fn new(
        author_raw: &str,
        description_raw: &str,
        cfg: &Cfg,
        log: &mut Log,
    ) -> Result<Self> {
        let author = truncate_header_text("author", 32, author_raw, cfg, log)?;
        let description = truncate_header_text("description", 256, description_raw, cfg, log)?;
        Ok(Self {
            author,
            description,
        })
    }
}

macro_rules! make_helper_records {
    ($($type_simple:ident),+; $($type:ident),+) => {
#[derive(Default)]
pub struct HelperRecords {
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

fn get_plugin_info(path: PathBuf, id: usize) -> Result<PluginInfo> {
    match path.file_name() {
        Some(file_name) => {
            let name = file_name.to_string_lossy().into_owned();
            let name_low = name.to_lowercase();
            Ok(PluginInfo {
                id,
                name,
                name_low,
                path,
            })
        }
        None => Err(anyhow!(
            "Failed to get plugin name for \"{}\"",
            path.display()
        )),
    }
}
