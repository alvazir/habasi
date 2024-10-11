use crate::{msg, Cfg, Dial, Helper, Log, Out, Stats, StatsUpdateKind};
use anyhow::{anyhow, Context, Result};
use hashbrown::hash_map::Entry;
use tes3::esp::{DialogueInfo, ObjectFlags};

pub fn process(
    mut info: DialogueInfo,
    out: &mut Out,
    h: &mut Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let Some(active_dial_id) = h.l.active_dial_id else {
        return Err(anyhow!(
            "Failed to get dialogue id for info record \"{}\"",
            info.id
        ));
    };
    let out_dial = &mut out
        .dial
        .get_mut(active_dial_id)
        .with_context(|| {
            format!("Bug: out.dial doesn't contain active_dial_id = \"{active_dial_id}\"")
        })?
        .0;
    if out_dial.dialogue_type.info != info.data.dialogue_type
        && !info.flags.contains(ObjectFlags::DELETED)
    {
        if h.g.list_options.force_dial_type {
            change_dialogue_type(&mut info, out_dial, cfg, log)?;
        } else {
            return Err(error_dialogue_type(&info, out_dial));
        }
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

fn change_dialogue_type(
    info: &mut DialogueInfo,
    out_dial: &Dial,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let previous_dialogue_type = info.data.dialogue_type;
    info.data.dialogue_type = out_dial.dialogue_type.info;
    let text = format!(
        "Force dial type: \"{}\" info record's type \"{:?}\" was changed to \"{}\" dialogue's type \"{:?}\"",
        info.id,
        previous_dialogue_type,
        out_dial.dialogue.id,
        info.data.dialogue_type,
    );
    msg(text, 1, cfg, log)
}

fn error_dialogue_type(info: &DialogueInfo, dial: &Dial) -> anyhow::Error {
    anyhow!(
        "\nError: \"{}\" info record's type \"{:?}\" is different to \"{}\" dialogue's type \"{:?}\"\n\tOption \"--force-dial-type\" {}",
        info.id,
        info.data.dialogue_type,
        dial.dialogue.id,
        dial.dialogue.dialogue_type,
        if dial.dialogue_type.changed {
            "may help, though DIAL's type change is a sign of problematic plugin"
        } else {
            "should help, because it looks like the bug in plugin caused by OpenMW-CS 0.48-"
        }
    )
}
