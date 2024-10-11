use crate::{
    msg, Cfg, Helper, Log, Mode, Out, StatsUpdateKind, CRC64, SNDG_ID_MAX_LEN, SNDG_ID_SUFFIX_LEN,
    SNDG_MAX_SOUND_FLAG,
};
use anyhow::{anyhow, Context, Result};
use hashbrown::hash_map::Entry;
use std::fmt::Write as _;
use tes3::esp::{Plugin, SoundGen, StartScript, TES3Object};
mod cell;
mod dial;
mod header;
mod info;
mod land;
mod ltex;

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::wildcard_enum_match_arm
)]
pub fn process_records(
    plugin: Plugin,
    out: &mut Out,
    name: &str,
    h: &mut Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
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
                    let out_v = out.$type.get_mut(global_id).with_context(|| {
                        format!(
                            "Bug: indexing slicing out.{}[{global_id}]",
                            stringify!($type)
                        )
                    })?;
                    if out_v.0 != $value {
                        if !$simple || h.g.list_options.debug {
                            keep_previous!(out_v, $value);
                        }
                        out_v.0 = $value;
                        h.l.stats.$type(StatsUpdateKind::Replaced);
                    } else {
                        if h.g.list_options.debug {
                            keep_previous!(out_v, $value);
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
                0 => header::process(record, out, h)?,
                _ => match record {
                    TES3Object::Static(v) => {
                        if matches!(h.g.list_options.mode, Mode::Grass)
                            || h.g.list_options.turn_normal_grass
                        {
                            process!(stat, v, v.id.to_lowercase(), false);
                        }
                    }
                    TES3Object::Cell(cell) => cell::process(cell, out, name, h, cfg, log)?,
                    TES3Object::Header(_) => {
                        return Err(anyhow!("Plugin is invalid due to many header records"))
                    }
                    _ => continue,
                },
            }
        }
    } else {
        let mut land_found = false;
        for (count, record) in plugin.objects.into_iter().enumerate() {
            match count {
                0 => header::process(record, out, h)?,
                _ => match record {
                    TES3Object::Dialogue(dial) => dial::process(dial, out, h, cfg, log)?,
                    TES3Object::DialogueInfo(info) => info::process(info, out, h, cfg, log)?,
                    _ => {
                        if h.l.active_dial_id.is_some() {
                            h.l.active_dial_id = None;
                        };
                        match record {
                            TES3Object::GameSetting(v) => {
                                process!(gmst, v, v.id.to_lowercase(), true);
                            }
                            TES3Object::GlobalVariable(v) => {
                                process!(glob, v, v.id.to_lowercase(), true);
                            }
                            TES3Object::Class(v) => process!(clas, v, v.id.to_lowercase(), false),
                            TES3Object::Faction(v) => process!(fact, v, v.id.to_lowercase(), true),
                            TES3Object::Race(v) => process!(race, v, v.id.to_lowercase(), false),
                            TES3Object::Sound(v) => process!(soun, v, v.id.to_lowercase(), false),
                            TES3Object::SoundGen(mut v) => {
                                assign_id_to_sndg_with_empty_id(&mut v, cfg, log)?;
                                process!(sndg, v, v.id.to_lowercase(), false);
                            }
                            TES3Object::Skill(v) => process!(skil, v, v.skill_id, false),
                            TES3Object::MagicEffect(v) => process!(mgef, v, v.effect_id, false),
                            TES3Object::Script(v) => process!(scpt, v, v.id.to_lowercase(), true),
                            TES3Object::Region(v) => process!(regn, v, v.id.to_lowercase(), true),
                            TES3Object::Birthsign(v) => {
                                process!(bsgn, v, v.id.to_lowercase(), false);
                            }
                            TES3Object::StartScript(mut v) => {
                                assign_id_to_sscr_with_empty_id(&mut v, cfg, log)?;
                                process!(sscr, v, v.id.to_lowercase(), true);
                            }
                            TES3Object::LandscapeTexture(ltex) => {
                                ltex::process(ltex, land_found, out, h)?;
                            }
                            TES3Object::Spell(v) => process!(spel, v, v.id.to_lowercase(), false),
                            TES3Object::Static(v) => process!(stat, v, v.id.to_lowercase(), false),
                            TES3Object::Door(v) => process!(door, v, v.id.to_lowercase(), false),
                            TES3Object::MiscItem(v) => {
                                process!(misc, v, v.id.to_lowercase(), false);
                            }
                            TES3Object::Weapon(v) => process!(weap, v, v.id.to_lowercase(), false),
                            TES3Object::Container(v) => {
                                process!(cont, v, v.id.to_lowercase(), false);
                            }
                            TES3Object::Creature(v) => {
                                process!(crea, v, v.id.to_lowercase(), false);
                            }
                            TES3Object::Bodypart(v) => {
                                process!(body, v, v.id.to_lowercase(), false);
                            }
                            TES3Object::Light(v) => process!(ligh, v, v.id.to_lowercase(), false),
                            TES3Object::Enchanting(v) => {
                                process!(ench, v, v.id.to_lowercase(), false);
                            }
                            TES3Object::Npc(v) => process!(npc_, v, v.id.to_lowercase(), false),
                            TES3Object::Armor(v) => process!(armo, v, v.id.to_lowercase(), false),
                            TES3Object::Clothing(v) => {
                                process!(clot, v, v.id.to_lowercase(), false);
                            }
                            TES3Object::RepairItem(v) => {
                                process!(repa, v, v.id.to_lowercase(), false);
                            }
                            TES3Object::Activator(v) => {
                                process!(acti, v, v.id.to_lowercase(), false);
                            }
                            TES3Object::Apparatus(v) => {
                                process!(appa, v, v.id.to_lowercase(), false);
                            }
                            TES3Object::Lockpick(v) => {
                                process!(lock, v, v.id.to_lowercase(), false);
                            }
                            TES3Object::Probe(v) => process!(prob, v, v.id.to_lowercase(), false),
                            TES3Object::Ingredient(v) => {
                                process!(ingr, v, v.id.to_lowercase(), false);
                            }
                            TES3Object::Book(v) => process!(book, v, v.id.to_lowercase(), false),
                            TES3Object::Alchemy(v) => process!(alch, v, v.id.to_lowercase(), false),
                            TES3Object::LeveledItem(v) => {
                                process!(levi, v, v.id.to_lowercase(), false);
                            }
                            TES3Object::LeveledCreature(v) => {
                                process!(levc, v, v.id.to_lowercase(), false);
                            }
                            TES3Object::Cell(cell) => cell::process(cell, out, name, h, cfg, log)?,
                            TES3Object::Landscape(land) => {
                                land::process(land, &mut land_found, out, h)?;
                            }
                            TES3Object::PathGrid(v) => {
                                process!(pgrd, v, v.cell.to_lowercase(), true);
                            }
                            TES3Object::Header(_) => {
                                return Err(anyhow!("Plugin is invalid due to many header records"))
                            }
                            _ => continue,
                        }
                    }
                },
            }
        }
    }
    Ok(())
}

macro_rules! keep_previous {
    ($out_tuple:ident, $value:expr) => {
        if $out_tuple.1.is_empty() {
            $out_tuple.1.push($out_tuple.0.clone());
        }
        $out_tuple.1.push($value.clone());
    };
}

pub(in crate::input) use keep_previous;

pub fn assign_id_to_sscr_with_empty_id(
    sscr: &mut StartScript,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    if sscr.id.is_empty() {
        sscr.id = CRC64.checksum(sscr.script.as_bytes()).to_string();
        let text = format!(
            "    SSCR with empty id(Script:\"{}\") was assigned id \"{}\"",
            sscr.script, sscr.id
        );
        msg(text, 2, cfg, log)?;
    }
    Ok(())
}

pub fn assign_id_to_sndg_with_empty_id(
    sndg: &mut SoundGen,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    if sndg.id.is_empty() {
        let mut text = format!(
            "    SNDG with empty id(Creature_ID:\"{}\", Sound_ID:\"{}\") was ",
            sndg.creature, sndg.sound
        );
        #[allow(clippy::as_conversions)]
        let sndg_type = sndg.sound_gen_type as u32;
        if sndg_type > SNDG_MAX_SOUND_FLAG {
            write!(
                text,
                "NOT assigned id due to unknown Type(sound_flags \"{sndg_type}\" > \"{SNDG_MAX_SOUND_FLAG}\")"
            )?;
        } else {
            let sndg_creature_truncated =
                if sndg.creature.len() > (SNDG_ID_MAX_LEN - SNDG_ID_SUFFIX_LEN) {
                    sndg.creature
                        .get(..SNDG_ID_MAX_LEN - SNDG_ID_SUFFIX_LEN)
                        .with_context(|| {
                            format!(
                                "Bug: indexing slicing sndg.creature[..{}]",
                                SNDG_ID_MAX_LEN - SNDG_ID_SUFFIX_LEN
                            )
                        })?
                } else {
                    &*sndg.creature
                };
            sndg.id = format!("{sndg_creature_truncated}{sndg_type:>0SNDG_ID_SUFFIX_LEN$}");
            write!(text, "assigned id \"{}\"", sndg.id)?;
        }
        msg(text, 2, cfg, log)?;
    }
    Ok(())
}
