use super::keep_previous;
use crate::{GlobalVtexId, Helper, LocalVtexId, Out, StatsUpdateKind};
use anyhow::{anyhow, Result};
use hashbrown::{hash_map::Entry, HashMap};
use tes3::esp::{Landscape, TextureIndices};

pub(crate) fn process_land(land: Landscape, land_found: &mut bool, out: &mut Out, h: &mut Helper) -> Result<()> {
    if !*land_found {
        *land_found = true
    };
    match h.g.r.land.entry(land.grid) {
        Entry::Vacant(v) => {
            let land_len = out.land.len();
            out.land.push((
                Landscape {
                    texture_indices: update_texture_indices(land.texture_indices, &h.l.vtex)?,
                    ..land
                },
                Vec::new(),
            ));
            v.insert(land_len);
            h.l.stats.land(StatsUpdateKind::Processed);
        }
        Entry::Occupied(o) => {
            let land_global_id = *o.get();
            let new_land = Landscape {
                texture_indices: update_texture_indices(land.texture_indices, &h.l.vtex)?,
                ..land
            };
            let out_v = &mut out.land[land_global_id];
            if out_v.0 != new_land {
                keep_previous!(out_v, new_land);
                out_v.0 = new_land;
                h.l.stats.land(StatsUpdateKind::Replaced);
            } else {
                if h.g.list_options.debug {
                    keep_previous!(out_v, new_land);
                }
                h.l.stats.land(StatsUpdateKind::Duplicate);
            }
        }
    };
    Ok(())
}

// COMMENT: VTEX id 1 corresponds to LTEX with id 0, VTEX id 0 is some "default" texture
fn update_texture_indices(mut vtex: TextureIndices, vtex_map: &HashMap<LocalVtexId, GlobalVtexId>) -> Result<TextureIndices> {
    for line in vtex.data.iter_mut() {
        for id in line {
            if *id != 0 {
                *id = match vtex_map.get(id) {
                    None => return Err(anyhow!("Error: there is no such VTEX id in vtex_map")),
                    Some(id) => *id,
                };
            }
        }
    }
    Ok(vtex)
}
