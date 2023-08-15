use crate::{Cfg, Helper, Log, Mode, Out, StatsUpdateKind};
use anyhow::{anyhow, Result};
use hashbrown::hash_map::Entry;
use tes3::esp::{Plugin, TES3Object};
mod cell;
mod dial;
mod header;
mod info;
mod land;
mod ltex;
use cell::process_cell;
use dial::process_dial;
use header::process_header;
use info::process_info;
use land::process_land;
use ltex::process_ltex;

pub(crate) fn process_records(plugin: Plugin, out: &mut Out, name: &str, h: &mut Helper, cfg: &Cfg, log: &mut Log) -> Result<()> {
    macro_rules! process {
        ($type:ident, $value:expr, $key:expr, $simple:expr) => {
            match h.g.r.$type.entry($key) {
                Entry::Vacant(v) => {
                    let len = out.$type.len();
                    out.$type.push(($value, Vec::new()));
                    v.insert(len);
                    h.l.stats.$type(StatsUpdateKind::Processed);
                }
                Entry::Occupied(o) => {
                    let global_id = *o.get();
                    let out_v = &mut out.$type[global_id];
                    if out_v.0 != $value {
                        if !$simple || h.g.list_options.debug {
                            if out_v.1.is_empty() {
                                out_v.1.push(out_v.0.clone());
                            }
                            out_v.1.push($value.clone());
                        }
                        out_v.0 = $value;
                        h.l.stats.$type(StatsUpdateKind::Replaced);
                    } else {
                        if h.g.list_options.debug {
                            if out_v.1.is_empty() {
                                out_v.1.push(out_v.0.clone());
                            }
                            out_v.1.push($value.clone());
                        }
                        h.l.stats.$type(StatsUpdateKind::Duplicate);
                    }
                }
            }
        };
    }
    if h.g.list_options.insufficient_merge {
        for (count, record) in plugin.objects.into_iter().enumerate() {
            match count {
                0 => process_header(record, out, h)?,
                _ => match record {
                    TES3Object::Static(v) => {
                        if matches!(h.g.list_options.mode, Mode::Grass) || h.g.list_options.turn_normal_grass {
                            process!(stat, v, v.id.to_lowercase(), false);
                        }
                    }
                    TES3Object::Cell(cell) => process_cell(cell, out, name, h, cfg, log)?,
                    TES3Object::Header(_) => return Err(anyhow!("Plugin is invalid due to many header records")),
                    _ => continue,
                },
            }
        }
    } else {
        let mut land_found = false;
        for (count, record) in plugin.objects.into_iter().enumerate() {
            match count {
                0 => process_header(record, out, h)?,
                _ => match record {
                    TES3Object::Dialogue(dial) => process_dial(dial, out, h),
                    TES3Object::DialogueInfo(info) => process_info(info, out, h, cfg)?,
                    _ => {
                        if h.l.active_dial_id.is_some() {
                            h.l.active_dial_id = None
                        };
                        match record {
                            TES3Object::GameSetting(v) => process!(gmst, v, v.id.to_lowercase(), true),
                            TES3Object::GlobalVariable(v) => process!(glob, v, v.id.to_lowercase(), true),
                            TES3Object::Class(v) => process!(clas, v, v.id.to_lowercase(), false),
                            TES3Object::Faction(v) => process!(fact, v, v.id.to_lowercase(), true),
                            TES3Object::Race(v) => process!(race, v, v.id.to_lowercase(), false),
                            TES3Object::Sound(v) => process!(soun, v, v.id.to_lowercase(), false),
                            TES3Object::SoundGen(v) => process!(sndg, v, v.id.to_lowercase(), false),
                            TES3Object::Skill(v) => process!(skil, v, v.skill_id, false),
                            TES3Object::MagicEffect(v) => process!(mgef, v, v.effect_id, false),
                            TES3Object::Script(v) => process!(scpt, v, v.id.to_lowercase(), true),
                            TES3Object::Region(v) => process!(regn, v, v.id.to_lowercase(), true),
                            TES3Object::Birthsign(v) => process!(bsgn, v, v.id.to_lowercase(), false),
                            TES3Object::StartScript(v) => process!(sscr, v, v.id.to_lowercase(), true),
                            TES3Object::LandscapeTexture(ltex) => process_ltex(ltex, &land_found, out, h)?,
                            TES3Object::Spell(v) => process!(spel, v, v.id.to_lowercase(), false),
                            TES3Object::Static(v) => process!(stat, v, v.id.to_lowercase(), false),
                            TES3Object::Door(v) => process!(door, v, v.id.to_lowercase(), false),
                            TES3Object::MiscItem(v) => process!(misc, v, v.id.to_lowercase(), false),
                            TES3Object::Weapon(v) => process!(weap, v, v.id.to_lowercase(), false),
                            TES3Object::Container(v) => process!(cont, v, v.id.to_lowercase(), false),
                            TES3Object::Creature(v) => process!(crea, v, v.id.to_lowercase(), false),
                            TES3Object::Bodypart(v) => process!(body, v, v.id.to_lowercase(), false),
                            TES3Object::Light(v) => process!(ligh, v, v.id.to_lowercase(), false),
                            TES3Object::Enchanting(v) => process!(ench, v, v.id.to_lowercase(), false),
                            TES3Object::Npc(v) => process!(npc_, v, v.id.to_lowercase(), false),
                            TES3Object::Armor(v) => process!(armo, v, v.id.to_lowercase(), false),
                            TES3Object::Clothing(v) => process!(clot, v, v.id.to_lowercase(), false),
                            TES3Object::RepairItem(v) => process!(repa, v, v.id.to_lowercase(), false),
                            TES3Object::Activator(v) => process!(acti, v, v.id.to_lowercase(), false),
                            TES3Object::Apparatus(v) => process!(appa, v, v.id.to_lowercase(), false),
                            TES3Object::Lockpick(v) => process!(lock, v, v.id.to_lowercase(), false),
                            TES3Object::Probe(v) => process!(prob, v, v.id.to_lowercase(), false),
                            TES3Object::Ingredient(v) => process!(ingr, v, v.id.to_lowercase(), false),
                            TES3Object::Book(v) => process!(book, v, v.id.to_lowercase(), false),
                            TES3Object::Alchemy(v) => process!(alch, v, v.id.to_lowercase(), false),
                            TES3Object::LeveledItem(v) => process!(levi, v, v.id.to_lowercase(), false),
                            TES3Object::LeveledCreature(v) => process!(levc, v, v.id.to_lowercase(), false),
                            TES3Object::Cell(cell) => process_cell(cell, out, name, h, cfg, log)?,
                            TES3Object::Landscape(land) => process_land(land, &mut land_found, out, h)?,
                            TES3Object::PathGrid(v) => process!(pgrd, v, v.cell.to_lowercase(), true),
                            TES3Object::Header(_) => return Err(anyhow!("Plugin is invalid due to many header records")),
                            _ => continue,
                        }
                    }
                },
            }
        }
    }
    Ok(())
}
