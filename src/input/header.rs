use crate::{
    increment, GlobalMaster, Helper, LocalMaster, LocalMergedMaster, MasterNameLow, Out,
    StatsUpdateKind,
};
use anyhow::{anyhow, Context as _, Result};
use tes3::esp::TES3Object;

pub fn process(record: TES3Object, out: &mut Out, h: &mut Helper) -> Result<()> {
    let TES3Object::Header(header) = record else {
        return Err(anyhow!("Plugin's first record is not a header"));
    };
    for (&(ref master_name, master_size), id) in header.masters.iter().zip(1_u32..) {
        let name_low: MasterNameLow = master_name.to_lowercase();
        match h
            .g
            .plugins_processed
            .iter()
            .find(|x| x.name_low == name_low)
        {
            Some(_) => h.l.merged_masters.push(LocalMergedMaster {
                local_id: id,
                name_low,
            }),
            None => match h.g.masters.iter().find(|x| x.name_low == name_low) {
                None => {
                    let global_master_id = u32::try_from(h.g.masters.len()).with_context(|| {
                        format!(
                            "Bug: failed to cast {:?}(h.g.masters.len(), usize) to u32",
                            h.g.masters.len()
                        )
                    })?;
                    let next_global_master_id = increment!(global_master_id);
                    h.l.masters.push(LocalMaster {
                        local_id: id,
                        global_id: next_global_master_id,
                    });
                    h.g.masters.push(GlobalMaster {
                        global_id: next_global_master_id,
                        name_low,
                    });
                    out.masters.push((master_name.to_owned(), master_size));
                }
                Some(global_master) => {
                    h.l.masters.push(LocalMaster {
                        local_id: id,
                        global_id: global_master.global_id,
                    });
                    match out
                        .masters
                        .get_mut(usize::try_from(global_master.global_id)?.checked_sub(1).with_context(|| {
                            format!(
                                "Bug: overflow decrementing global_master.global_id = \"{}\"",
                                global_master.global_id
                            )
                        })?) {
                        None => {
                            return Err(anyhow!(
                                "Error: Failed to find master \"{}\" with id \"{}\", bacause masters list length is \"{}\"",
                                name_low,
                                global_master.global_id,
                                out.masters.len()
                            ))
                        }
                        Some(&mut (_, ref mut old_master_size)) => {
                            if master_size != *old_master_size {
                                *old_master_size = master_size;
                            }
                        }
                    };
                }
            },
        }
    }
    h.l.stats.tes3(StatsUpdateKind::Merged);
    Ok(())
}
