use crate::{GlobalMaster, Helper, LocalMaster, LocalMergedMaster, MasterNameLow, Out, StatsUpdateKind};
use anyhow::{anyhow, Result};
use tes3::esp::Header;

pub(crate) fn process_header(header: Header, out: &mut Out, h: &mut Helper) -> Result<()> {
    for ((master_name, master_size), id) in header.masters.iter().zip(1u32..) {
        let name_low: MasterNameLow = master_name.to_lowercase();
        if h.g.plugins_processed.contains(&name_low) {
            h.l.merged_masters.push(LocalMergedMaster { local_id: id, name_low });
        } else {
            match h.g.masters.iter().find(|x| x.name_low == name_low) {
                None => {
                    let next_global_master_id = h.g.masters.len() as u32 + 1;
                    h.l.masters.push(LocalMaster {
                        local_id: id,
                        global_id: next_global_master_id,
                    });
                    h.g.masters.push(GlobalMaster {
                        global_id: next_global_master_id,
                        name_low,
                    });
                    out.masters.push((master_name.to_owned(), *master_size));
                }
                Some(global_master) => {
                    h.l.masters.push(LocalMaster {
                        local_id: id,
                        global_id: global_master.global_id,
                    });
                    match out.masters.get_mut(global_master.global_id as usize - 1) {
                        None => {
                            return Err(anyhow!(
                                "Error: Failed to find master \"{}\" with id \"{}\", bacause masters list length is \"{}\"",
                                name_low,
                                global_master.global_id,
                                out.masters.len()
                            ))
                        }
                        Some((_, old_master_size)) => {
                            if master_size != old_master_size {
                                *old_master_size = *master_size
                            }
                        }
                    };
                }
            }
        }
    }
    h.l.stats.tes3(StatsUpdateKind::Merged);
    Ok(())
}
