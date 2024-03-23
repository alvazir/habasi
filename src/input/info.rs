use crate::{Cfg, Dial, Helper, Out, Stats, StatsUpdateKind};
use anyhow::{anyhow, Context, Result};
use hashbrown::hash_map::Entry;
use tes3::esp::{DialogueInfo, ObjectFlags};

pub fn process(info: DialogueInfo, out: &mut Out, h: &mut Helper, cfg: &Cfg) -> Result<()> {
    let active_dial_id = match h.l.active_dial_id {
        None => {
            return Err(anyhow!(
                "Failed to get dialogue id for info record \"{}\"",
                info.id
            ))
        }
        Some(active_dial_id) => active_dial_id,
    };
    let out_dial = &mut out
        .dial
        .get_mut(active_dial_id)
        .with_context(|| {
            format!("Bug: out.dial doesn't contain active_dial_id = \"{active_dial_id}\"")
        })?
        .0;
    #[allow(clippy::as_conversions)]
    if out_dial.dialogue.dialogue_type as u8
        != u8::try_from(info.data.dialogue_type as u32).with_context(|| {
            format!(
                "Bug: failed to cast {}(info.data.dialogue_type, DialogueType as u32) to u8",
                info.data.dialogue_type as u32
            )
        })?
        && !info.flags.contains(ObjectFlags::DELETED)
    {
        return Err(anyhow!(
            "Error: \"{}\" info record's kind is different to \"{}\" dialogue's",
            info.id,
            out_dial.dialogue.id,
        ));
    }
    let next_info_id = out_dial.info.len();
    match h.g.r.dials.get_mut(&h.l.active_dial_name_low) {
        None => {
            return Err(anyhow!(
                "Unreachable error: dial id for info is not found in list of dial ids"
            ))
        }
        Some(dial_meta) => match dial_meta.info_metas.entry(info.id.clone()) {
            Entry::Vacant(v) => {
                v.insert(next_info_id);
                h.l.stats.info(StatsUpdateKind::Processed);
            }
            Entry::Occupied(mut o) => {
                if &info
                    == out_dial.info.get(*o.get()).with_context(|| {
                        format!("Bug: indexing slicing out_dial.info[{}]", *o.get())
                    })?
                {
                    h.l.stats.info(StatsUpdateKind::Duplicate);
                    return Ok(());
                }
                if cfg.advanced.keep_only_last_info_ids.contains_key(&info.id) {
                    exclude_info(&info.id, out_dial, &mut h.l.stats);
                }
                let value = o.get_mut();
                *value = next_info_id;
                h.l.stats.info(StatsUpdateKind::Processed);
            }
        },
    };
    out_dial.info.push(info);
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
