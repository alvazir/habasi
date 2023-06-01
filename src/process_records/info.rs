use crate::{Helper, Out, StatsUpdateKind};
use anyhow::{anyhow, Result};
use tes3::esp::{DialogueInfo, ObjectFlags};

pub(crate) fn process_info(info: DialogueInfo, out: &mut Out, h: &mut Helper) -> Result<()> {
    let active_dial_id = match h.l.active_dial_id {
        None => return Err(anyhow!("Failed to get dialogue id for info record \"{}\"", info.id)),
        Some(active_dial_id) => active_dial_id,
    };
    if out.dial[active_dial_id].0.dialogue.kind as u8 != info.data.kind as u8 && !info.flags.contains(ObjectFlags::DELETED) {
        return Err(anyhow!(
            "Error: \"{}\" info record's kind is different to \"{}\" dialogue's",
            info.id,
            out.dial[active_dial_id].0.dialogue.id,
        ));
    }
    let next_info_id = out.dial[active_dial_id].0.info.len();
    use hashbrown::hash_map::Entry;
    match h.g.r.dials.get_mut(&h.l.active_dial_name_low) {
        None => return Err(anyhow!("Unreachable error: dial id for info is not found in list of dial ids")),
        Some(dial_meta) => match dial_meta.info_metas.entry(info.id.clone()) {
            Entry::Vacant(v) => {
                v.insert(next_info_id);
                h.l.stats.info(StatsUpdateKind::Processed);
            }
            Entry::Occupied(mut o) => {
                if out.dial[active_dial_id].0.info[*o.get()] == info {
                    h.l.stats.info(StatsUpdateKind::Duplicate);
                    return Ok(());
                } else {
                    let value = o.get_mut();
                    *value = next_info_id;
                    h.l.stats.info(StatsUpdateKind::Processed);
                }
            }
        },
    };
    out.dial[active_dial_id].0.info.push(info);
    Ok(())
}
