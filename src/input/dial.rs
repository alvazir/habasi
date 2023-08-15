use crate::{Dial, DialMeta, Helper, Out, StatsUpdateKind};
use hashbrown::{hash_map::Entry, HashMap};
use tes3::esp::Dialogue;

pub(crate) fn process_dial(dial: Dialogue, out: &mut Out, h: &mut Helper) {
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
            if out.dial[active_dial_id].0.dialogue != dial {
                out.dial[active_dial_id].0.dialogue = dial;
                h.l.stats.dial(StatsUpdateKind::Replaced);
            } else {
                h.l.stats.dial(StatsUpdateKind::Duplicate);
            }
            h.l.active_dial_id = Some(o.get().global_dial_id);
        }
    };
    h.l.active_dial_name_low = dial_id_low;
}
