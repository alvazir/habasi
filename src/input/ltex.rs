use crate::{Helper, Out, StatsUpdateKind};
use anyhow::{anyhow, Context, Result};
use hashbrown::hash_map::Entry;
use tes3::esp::LandscapeTexture;

pub fn process(ltex: LandscapeTexture, land_found: bool, out: &mut Out, h: &mut Helper) -> Result<()> {
    if land_found {
        return Err(anyhow!("Plugin is corrupted, because LTEX record comes after LAND records"));
    }
    let ltex_index =
        u16::try_from(ltex.index).with_context(|| format!("Bug: failed to cast {:?}(ltex.index, u32) to u16", ltex.index))?;
    match h.g.r.ltex.entry(ltex.id.to_lowercase()) {
        Entry::Vacant(v) => {
            let ltex_len = u16::try_from(out.ltex.len()).with_context(|| {
                format!(
                    "Bug: failed to cast {:?}(out.ltex.len(), usize) to u16",
                    h.g.plugins_processed.len()
                )
            })?;
            if h.l
                .vtex
                .insert(
                    ltex_index
                        .checked_add(1)
                        .with_context(|| format!("Bug: overflow incrementing ltex_index = \"{ltex_index}\""))?,
                    ltex_len
                        .checked_add(1)
                        .with_context(|| format!("Bug: overflow incrementing ltex_len = \"{ltex_len}\""))?,
                )
                .is_some()
            {
                return Err(anyhow!("Error: there is already vtex pair for this plugin"));
            };
            out.ltex.push((
                LandscapeTexture {
                    index: u32::from(ltex_len),
                    ..ltex
                },
                Vec::new(),
            ));
            v.insert(usize::from(ltex_len));
            h.l.stats.ltex(StatsUpdateKind::Processed);
        }
        Entry::Occupied(o) => {
            let mut replaced = false;
            let ltex_global_id = *o.get();
            let ltex_global_id_incremented = ltex_global_id
                .checked_add(1)
                .with_context(|| format!("Bug: overflow incrementing ltex_global_id = \"{ltex_global_id}\""))?;
            if h.l
                .vtex
                .insert(
                    ltex_index
                        .checked_add(1)
                        .with_context(|| format!("Bug: overflow incrementing ltex_index = \"{ltex_index}\""))?,
                    u16::try_from(ltex_global_id_incremented).with_context(|| {
                        format!("Bug: failed to cast \"{ltex_global_id_incremented}\"(ltex_global_id_incremented, usize) to u16")
                    })?,
                )
                .is_some()
            {
                return Err(anyhow!("Error: there is already vtex pair for this plugin"));
            };
            let out_ltex = out
                .ltex
                .get_mut(ltex_global_id)
                .with_context(|| format!("Bug: indexing slicing out.ltex[{ltex_global_id}]"))?;
            if out_ltex.0.flags != ltex.flags {
                out_ltex.0.flags = ltex.flags;
                replaced = true;
            }
            if out_ltex.0.file_name.to_lowercase() != ltex.file_name.to_lowercase() {
                out_ltex.0.file_name = ltex.file_name;
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
