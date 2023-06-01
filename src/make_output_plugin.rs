use crate::{msg, references_sorted, Cfg, Helper, Log, Mode, Out, StatsUpdateKind};
use anyhow::Result;
use hashbrown::HashMap;
use tes3::esp::{FixedString, Header, Plugin, Reference, TES3Object};

pub(crate) fn make_output_plugin(
    name: &str,
    mut out: Out,
    out_plugin: &mut Plugin,
    h: &mut Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let mut objects = Vec::new();
    out.skil.sort_by_key(|x| x.0.skill_id as i32);
    out.mgef.sort_by_key(|x| x.0.effect_id as i32);
    if h.g.reindex {
        let mut refr = 1u32;
        for (last, _) in out.cell.iter_mut() {
            let mut new_refs: HashMap<(u32, u32), Reference> = HashMap::new();
            let mut references: Vec<&Reference> = last.references.values().collect();
            references_sorted(&mut references);
            for reference in references {
                if reference.mast_index == 0 {
                    let new_ref = Reference {
                        refr_index: refr,
                        ..reference.clone()
                    };
                    new_refs.insert((0u32, refr), new_ref);
                    refr += 1;
                } else {
                    new_refs.insert((reference.mast_index, reference.refr_index), reference.clone());
                }
            }
            last.references = new_refs;
        }
        let text = format!("Output plugin \"{}\": references reindexed", name);
        msg(text, 1, cfg, log)?;
    }
    macro_rules! move_out {
        ($type:ident, $obj:ident, $mode:expr) => {
            let type_str = stringify!($type);
            for (last, prevs) in out.$type.into_iter() {
                let prevs_len = prevs.len();
                if prevs_len > 0 {
                    if h.g.debug {
                        h.g.stats.$type(StatsUpdateKind::ResultMergeableUnique);
                        for prev in prevs {
                            objects.push(TES3Object::$obj(prev));
                            h.g.stats.$type(StatsUpdateKind::ResultMergeableTotal);
                        }
                    } else {
                        match $mode {
                            Mode::Replace | Mode::CompleteReplace | Mode::Grass => {}
                            _ => {
                                match $mode {
                                    Mode::KeepWithoutLands | Mode::JobashaWithoutLands if type_str == "land" => {}
                                    // COMMENT: Mode::Jobasha | Mode::JobashaWithoutLands if type_str == "gmst" || type_str == "clas" => {}
                                    _ => {
                                        h.g.stats.$type(StatsUpdateKind::ResultMergeableUnique);
                                        for (count, prev) in prevs.into_iter().enumerate() {
                                            if count < prevs_len - 1 {
                                                objects.push(TES3Object::$obj(prev));
                                                h.g.stats.$type(StatsUpdateKind::ResultMergeableTotal);
                                            }
                                        }
                                    }
                                };
                            }
                        }
                    }
                }
                objects.push(TES3Object::$obj(last));
                h.g.stats.$type(StatsUpdateKind::ResultUnique);
            }
        };
    }
    move_out!(gmst, GameSetting, h.g.mode);
    move_out!(glob, GlobalVariable, h.g.mode);
    move_out!(clas, Class, h.g.mode);
    move_out!(fact, Faction, h.g.mode);
    move_out!(race, Race, h.g.mode);
    move_out!(soun, Sound, h.g.mode);
    move_out!(sndg, SoundGen, h.g.mode);
    move_out!(skil, Skill, h.g.mode);
    move_out!(mgef, MagicEffect, h.g.mode);
    move_out!(scpt, Script, h.g.mode);
    move_out!(regn, Region, h.g.mode);
    move_out!(bsgn, Birthsign, h.g.mode);
    move_out!(sscr, StartScript, h.g.mode);
    move_out!(ltex, LandscapeTexture, h.g.mode);
    move_out!(spel, Spell, h.g.mode);
    move_out!(stat, Static, h.g.mode);
    move_out!(door, Door, h.g.mode);
    move_out!(misc, MiscItem, h.g.mode);
    move_out!(weap, Weapon, h.g.mode);
    move_out!(cont, Container, h.g.mode);
    move_out!(crea, Creature, h.g.mode);
    move_out!(body, Bodypart, h.g.mode);
    move_out!(ligh, Light, h.g.mode);
    move_out!(ench, Enchanting, h.g.mode);
    move_out!(npc_, Npc, h.g.mode);
    move_out!(armo, Armor, h.g.mode);
    move_out!(clot, Clothing, h.g.mode);
    move_out!(repa, RepairItem, h.g.mode);
    move_out!(acti, Activator, h.g.mode);
    move_out!(appa, Apparatus, h.g.mode);
    move_out!(lock, Lockpick, h.g.mode);
    move_out!(prob, Probe, h.g.mode);
    move_out!(ingr, Ingredient, h.g.mode);
    move_out!(book, Book, h.g.mode);
    move_out!(alch, Alchemy, h.g.mode);
    let lev_mode = if let Mode::CompleteReplace = h.g.mode {
        Mode::CompleteReplace
    } else {
        Mode::Keep
    };
    move_out!(levi, LeveledItem, lev_mode);
    move_out!(levc, LeveledCreature, lev_mode);
    move_out!(cell, Cell, h.g.mode);
    move_out!(land, Landscape, h.g.mode);
    move_out!(pgrd, PathGrid, h.g.mode);
    for dial in out.dial.into_iter() {
        objects.push(TES3Object::Dialogue(dial.0.dialogue));
        h.g.stats.dial(StatsUpdateKind::ResultUnique);
        for info in dial.0.info {
            objects.push(TES3Object::DialogueInfo(info));
            h.g.stats.info(StatsUpdateKind::ResultUnique);
        }
    }
    let header = make_header(name, out.masters, h.g.stats.total(), h.g.strip_masters, cfg, log)?;
    out_plugin.objects.push(TES3Object::Header(header));
    out_plugin.objects.extend(objects);
    Ok(())
}

fn make_header(
    name: &str,
    masters: Vec<(String, u64)>,
    num_objects: u32,
    strip_masters: bool,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<Header> {
    let masters = match strip_masters {
        true => {
            let text = format!("Output plugin \"{}\": master subrecords stripped from header", name);
            msg(text, 1, cfg, log)?;
            Vec::new()
        }
        false => masters,
    };
    Ok(Header {
        version: cfg.guts.header_version,
        author: FixedString(String::from(&cfg.guts.header_author)),
        description: FixedString(String::from(&cfg.guts.header_description)),
        num_objects,
        masters,
        ..Default::default()
    })
}
