use crate::{
    get_cell_name, msg, select_header_description, show_removed_record_ids, Cfg, Dial, HeaderText, Helper, Log, Mode, Out,
    StatsUpdateKind,
};
use anyhow::Result;
use tes3::esp::{DialogueType2, FixedString, Header, ObjectFlags, Plugin, TES3Object};

#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
pub fn make_output_plugin(name: &str, out: Out, out_plugin: &mut Plugin, h: &mut Helper, cfg: &Cfg, log: &mut Log) -> Result<()> {
    let mut objects = Vec::new();
    let lev_mode = if matches!(h.g.list_options.mode, Mode::CompleteReplace) {
        Mode::CompleteReplace
    } else {
        Mode::Keep
    };
    let mut removed_record_ids = Vec::new();
    macro_rules! move_out {
        ($type:ident, $obj:ident, $mode:expr) => {
            let type_str = stringify!($type);
            for (last, prevs) in out.$type.into_iter() {
                let prevs_len = prevs.len();
                if h.g.list_options.exclude_deleted_records && last.flags.contains(ObjectFlags::DELETED) {
                    let removed_record_id = get_removed_record_id(TES3Object::$obj(last.clone()));
                    let text = format!("    Record {}: {removed_record_id} was excluded from the result due to \"DELETED\" flag", type_str.to_uppercase());
                    removed_record_ids.push(text);
                    h.g.stats.$type(StatsUpdateKind::Excluded);
                } else {
                    if prevs_len > 0 {
                        if h.g.list_options.debug {
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
                                                #[allow(clippy::arithmetic_side_effects)]
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
            }
        };
    }
    move_out!(gmst, GameSetting, h.g.list_options.mode);
    move_out!(glob, GlobalVariable, h.g.list_options.mode);
    move_out!(clas, Class, h.g.list_options.mode);
    move_out!(fact, Faction, h.g.list_options.mode);
    move_out!(race, Race, h.g.list_options.mode);
    move_out!(soun, Sound, h.g.list_options.mode);
    move_out!(sndg, SoundGen, h.g.list_options.mode);
    move_out!(skil, Skill, h.g.list_options.mode);
    move_out!(mgef, MagicEffect, h.g.list_options.mode);
    move_out!(scpt, Script, h.g.list_options.mode);
    move_out!(regn, Region, h.g.list_options.mode);
    move_out!(bsgn, Birthsign, h.g.list_options.mode);
    move_out!(sscr, StartScript, h.g.list_options.mode);
    move_out!(ltex, LandscapeTexture, h.g.list_options.mode);
    move_out!(spel, Spell, h.g.list_options.mode);
    move_out!(stat, Static, h.g.list_options.mode);
    move_out!(door, Door, h.g.list_options.mode);
    move_out!(misc, MiscItem, h.g.list_options.mode);
    move_out!(weap, Weapon, h.g.list_options.mode);
    move_out!(cont, Container, h.g.list_options.mode);
    move_out!(crea, Creature, h.g.list_options.mode);
    move_out!(body, Bodypart, h.g.list_options.mode);
    move_out!(ligh, Light, h.g.list_options.mode);
    move_out!(ench, Enchanting, h.g.list_options.mode);
    move_out!(npc_, Npc, h.g.list_options.mode);
    move_out!(armo, Armor, h.g.list_options.mode);
    move_out!(clot, Clothing, h.g.list_options.mode);
    move_out!(repa, RepairItem, h.g.list_options.mode);
    move_out!(acti, Activator, h.g.list_options.mode);
    move_out!(appa, Apparatus, h.g.list_options.mode);
    move_out!(lock, Lockpick, h.g.list_options.mode);
    move_out!(prob, Probe, h.g.list_options.mode);
    move_out!(ingr, Ingredient, h.g.list_options.mode);
    move_out!(book, Book, h.g.list_options.mode);
    move_out!(alch, Alchemy, h.g.list_options.mode);
    move_out!(levi, LeveledItem, lev_mode);
    move_out!(levc, LeveledCreature, lev_mode);
    move_out!(cell, Cell, h.g.list_options.mode);
    move_out!(land, Landscape, h.g.list_options.mode);
    move_out!(pgrd, PathGrid, h.g.list_options.mode);
    move_out_dial(out.dial, &mut objects, h);
    if h.g.list_options.exclude_deleted_records && !removed_record_ids.is_empty() {
        let reason = "\"exclude_deleted_records\" and DELETED record flag";
        show_removed_record_ids(&removed_record_ids, reason, name, 1, cfg, log)?;
    }
    let header_text = HeaderText::new(&cfg.guts.header_author, &select_header_description(h, cfg), cfg, log)?;
    let strip_masters = h.g.list_options.strip_masters;
    let header = make_header(name, out.masters, h.g.stats.total()?, strip_masters, header_text, cfg, log)?;
    out_plugin.objects.push(TES3Object::Header(header));
    out_plugin.objects.extend(objects);
    Ok(())
}

pub fn make_header(
    name: &str,
    mut masters: Vec<(String, u64)>,
    num_objects: u32,
    strip_masters: bool,
    header_text: HeaderText,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<Header> {
    if strip_masters {
        let text = format!("Output plugin {name:?}: master subrecords stripped from header");
        msg(text, 1, cfg, log)?;
        masters.clear();
    };
    Ok(Header {
        version: cfg.guts.header_version,
        author: FixedString(header_text.author),
        description: FixedString(header_text.description),
        num_objects,
        masters,
        ..Default::default()
    })
}

fn get_removed_record_id(tes3object: TES3Object) -> String {
    match tes3object {
        TES3Object::Header(v) => format!("{:?}", v.description),
        TES3Object::GameSetting(v) => format!("{:?}", v.id),
        TES3Object::GlobalVariable(v) => format!("{:?}", v.id),
        TES3Object::Class(v) => format!("{:?}", v.id),
        TES3Object::Faction(v) => format!("{:?}", v.id),
        TES3Object::Race(v) => format!("{:?}", v.id),
        TES3Object::Sound(v) => format!("{:?}", v.id),
        TES3Object::SoundGen(v) => format!("{:?}", v.id),
        TES3Object::Skill(v) => format!("{:?}", v.skill_id),
        TES3Object::MagicEffect(v) => format!("{:?}", v.effect_id),
        TES3Object::Script(v) => format!("{:?}", v.id),
        TES3Object::Region(v) => format!("{:?}", v.id),
        TES3Object::Birthsign(v) => format!("{:?}", v.id),
        TES3Object::StartScript(v) => format!("{:?}", v.id),
        TES3Object::LandscapeTexture(v) => format!("{:?}", v.id),
        TES3Object::Spell(v) => format!("{:?}", v.id),
        TES3Object::Static(v) => format!("{:?}", v.id),
        TES3Object::Door(v) => format!("{:?}", v.id),
        TES3Object::MiscItem(v) => format!("{:?}", v.id),
        TES3Object::Weapon(v) => format!("{:?}", v.id),
        TES3Object::Container(v) => format!("{:?}", v.id),
        TES3Object::Creature(v) => format!("{:?}", v.id),
        TES3Object::Bodypart(v) => format!("{:?}", v.id),
        TES3Object::Light(v) => format!("{:?}", v.id),
        TES3Object::Enchanting(v) => format!("{:?}", v.id),
        TES3Object::Npc(v) => format!("{:?}", v.id),
        TES3Object::Armor(v) => format!("{:?}", v.id),
        TES3Object::Clothing(v) => format!("{:?}", v.id),
        TES3Object::RepairItem(v) => format!("{:?}", v.id),
        TES3Object::Activator(v) => format!("{:?}", v.id),
        TES3Object::Apparatus(v) => format!("{:?}", v.id),
        TES3Object::Lockpick(v) => format!("{:?}", v.id),
        TES3Object::Probe(v) => format!("{:?}", v.id),
        TES3Object::Ingredient(v) => format!("{:?}", v.id),
        TES3Object::Book(v) => format!("{:?}", v.id),
        TES3Object::Alchemy(v) => format!("{:?}", v.id),
        TES3Object::LeveledItem(v) => format!("{:?}", v.id),
        TES3Object::LeveledCreature(v) => format!("{:?}", v.id),
        TES3Object::Cell(v) => get_cell_name(&v),
        TES3Object::Landscape(v) => format!("{:?}", v.grid),
        TES3Object::PathGrid(v) => format!("{:?}", v.cell),
        TES3Object::Dialogue(v) => format!("{:?}", v.id),
        TES3Object::DialogueInfo(v) => format!("{:?}", v.id),
    }
}

fn move_out_dial(out_dial: Vec<(Dial, Vec<Dial>)>, objects: &mut Vec<TES3Object>, h: &mut Helper) {
    let mut is_journal: bool;
    let mut journal = Vec::new();
    let mut non_journal = Vec::new();
    for dial in out_dial {
        if dial.0.dialogue.dialogue_type == DialogueType2::Journal {
            is_journal = true;
            journal.push(TES3Object::Dialogue(dial.0.dialogue));
        } else {
            is_journal = false;
            non_journal.push(TES3Object::Dialogue(dial.0.dialogue));
        };
        h.g.stats.dial(StatsUpdateKind::ResultUnique);
        for info in dial.0.info {
            if is_journal {
                journal.push(TES3Object::DialogueInfo(info));
            } else {
                non_journal.push(TES3Object::DialogueInfo(info));
            }
            h.g.stats.info(StatsUpdateKind::ResultUnique);
        }
    }
    objects.append(&mut journal);
    objects.append(&mut non_journal);
}
