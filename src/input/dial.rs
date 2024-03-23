use crate::{Dial, DialMeta, Helper, Out, StatsUpdateKind};
use anyhow::{Context, Result};
use hashbrown::{hash_map::Entry, HashMap};
use tes3::esp::Dialogue;

pub fn process(dial: Dialogue, out: &mut Out, h: &mut Helper) -> Result<()> {
    let dial_id_low = dial.id.to_lowercase();
    match h.g.r.dials.entry(dial_id_low.clone()) {
        Entry::Vacant(v) => {
            let dial_len = out.dial.len();
            v.insert(DialMeta {
                global_dial_id: dial_len,
                info_metas: HashMap::new(),
            });
            h.l.active_dial_id = Some(dial_len);
            out.dial.push((
                Dial {
                    dialogue: dial,
                    info: Vec::new(),
                    excluded_infos: Vec::new(),
                },
                Vec::new(),
            ));
            h.l.stats.dial(StatsUpdateKind::Processed);
        }
        Entry::Occupied(o) => {
            let active_dial_id = o.get().global_dial_id;
            h.l.active_dial_id = Some(active_dial_id);

            let out_dialogue = &mut out
                .dial
                .get_mut(active_dial_id)
                .with_context(|| format!("Bug: out.dial doesn't contain active_dial_id = \"{active_dial_id}\""))?
                .0
                .dialogue;
            if *out_dialogue == dial {
                h.l.stats.dial(StatsUpdateKind::Duplicate);
            } else {
                *out_dialogue = dial;
                h.l.stats.dial(StatsUpdateKind::Replaced);
            }
            h.l.active_dial_id = Some(o.get().global_dial_id);
        }
    };
    h.l.active_dial_name_low = dial_id_low;
    Ok(())
}
