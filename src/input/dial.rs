use crate::{msg, Cfg, Dial, DialMeta, Helper, Log, Out, StatsUpdateKind};
use anyhow::{Context as _, Result};
use hashbrown::{hash_map::Entry, HashMap};
use tes3::esp::Dialogue;

pub fn process(
    dial: Dialogue,
    out: &mut Out,
    h: &mut Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let dial_id_low = dial.id.to_lowercase();
    match h.g.r.dials.entry(dial_id_low.clone()) {
        Entry::Vacant(v) => {
            let dial_len = out.dial.len();
            v.insert(DialMeta {
                global_dial_id: dial_len,
                info_metas: HashMap::new(),
            });
            h.l.active_dial_id = Some(dial_len);
            out.dial.push((Dial::new(dial), Vec::new()));
            h.l.stats.dial(StatsUpdateKind::Processed);
        }
        Entry::Occupied(o) => {
            let active_dial_id = o.get().global_dial_id;
            h.l.active_dial_id = Some(active_dial_id);
            let out_0 = &mut out
                .dial
                .get_mut(active_dial_id)
                .with_context(|| {
                    format!("Bug: out.dial doesn't contain active_dial_id = \"{active_dial_id}\"")
                })?
                .0;
            let out_dialogue = &mut out_0.dialogue;
            if *out_dialogue == dial {
                h.l.stats.dial(StatsUpdateKind::Duplicate);
            } else {
                if out_dialogue.dialogue_type != dial.dialogue_type {
                    out_0.dialogue_type.change(dial.dialogue_type);
                    let text = format!(
                        "Warning: plugin \"{}\" changed \"{}\" dialogue's type from \"{:?}\" to \"{:?}\"",
                        h.l.plugin_info.name,
                        dial.id,
                        out_dialogue.dialogue_type,
                        dial.dialogue_type
                    );
                    msg(text, 0, cfg, log)?;
                }
                *out_dialogue = dial;
                h.l.stats.dial(StatsUpdateKind::Replaced);
            }
            h.l.active_dial_id = Some(o.get().global_dial_id);
        }
    };
    h.l.active_dial_name_low = dial_id_low;
    Ok(())
}
