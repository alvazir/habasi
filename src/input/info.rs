use crate::{Cfg, Dial, Helper, Out, Stats, StatsUpdateKind};
use anyhow::{anyhow, Result};
use hashbrown::hash_map::Entry;
use tes3::esp::{DialogueInfo, ObjectFlags};

pub(crate) fn process_info(info: DialogueInfo, out: &mut Out, h: &mut Helper, cfg: &Cfg) -> Result<()> {
    let active_dial_id = match h.l.active_dial_id {
        None => return Err(anyhow!("Failed to get dialogue id for info record \"{}\"", info.id)),
        Some(active_dial_id) => active_dial_id,
    };
    if out.dial[active_dial_id].0.dialogue.dialogue_type as u8 != info.data.dialogue_type as u8
        && !info.flags.contains(ObjectFlags::DELETED)
    {
        return Err(anyhow!(
            "Error: \"{}\" info record's kind is different to \"{}\" dialogue's",
            info.id,
            out.dial[active_dial_id].0.dialogue.id,
        ));
    }
    let next_info_id = out.dial[active_dial_id].0.info.len();
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
                    if cfg.advanced.keep_only_last_info_ids.contains_key(&info.id) {
                        exclude_info(&info.id, &mut out.dial[active_dial_id].0, &mut h.l.stats);
                    }
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

fn exclude_info(id: &str, dial: &mut Dial, stats: &mut Stats) {
    for (n, info) in dial.info.iter().enumerate() {
        if info.id == id && !dial.excluded_infos.contains(&n) {
            dial.excluded_infos.push(n);
            stats.info(StatsUpdateKind::Excluded);
        }
    }
}
