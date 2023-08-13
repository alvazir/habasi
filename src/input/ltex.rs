use crate::{Helper, Out, StatsUpdateKind};
use anyhow::{anyhow, Result};
use hashbrown::hash_map::Entry;
use tes3::esp::LandscapeTexture;

pub(crate) fn process_ltex(ltex: LandscapeTexture, land_found: &bool, out: &mut Out, h: &mut Helper) -> Result<()> {
    if *land_found {
        return Err(anyhow!("Plugin is corrupted, because LTEX record comes after LAND records"));
    }
    match h.g.r.ltex.entry(ltex.id.to_lowercase()) {
        Entry::Vacant(v) => {
            let ltex_len = out.ltex.len();
            if h.l.vtex.insert(ltex.index as u16 + 1, ltex_len as u16 + 1).is_some() {
                return Err(anyhow!("Error: there is already vtex pair for this plugin"));
            };
            out.ltex.push((
                LandscapeTexture {
                    index: ltex_len as u32,
                    ..ltex
                },
                Vec::new(),
            ));
            v.insert(ltex_len);
            h.l.stats.ltex(StatsUpdateKind::Processed);
        }
        Entry::Occupied(o) => {
            let mut replaced = false;
            let ltex_global_id = *o.get();
            if h.l.vtex.insert(ltex.index as u16 + 1, ltex_global_id as u16 + 1).is_some() {
                return Err(anyhow!("Error: there is already vtex pair for this plugin"));
            };
            if out.ltex[ltex_global_id].0.flags != ltex.flags {
                out.ltex[ltex_global_id].0.flags = ltex.flags;
                replaced = true;
            }
            if out.ltex[ltex_global_id].0.file_name.to_lowercase() != ltex.file_name.to_lowercase() {
                out.ltex[ltex_global_id].0.file_name = ltex.file_name;
                replaced = true;
            }
            if replaced {
                h.l.stats.ltex(StatsUpdateKind::Replaced);
            } else {
                h.l.stats.ltex(StatsUpdateKind::Merged);
            }
        }
    };
    Ok(())
}
