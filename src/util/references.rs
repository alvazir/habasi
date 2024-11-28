use super::{msg, Log};
use crate::{Cfg, Helper, IgnoredRefError, Out, PluginName};
use anyhow::{anyhow, Context as _, Result};
use std::fmt::Write as _;
use tes3::esp::Reference;

#[allow(clippy::module_name_repetitions)]
pub fn references_sorted(references: &mut [&Reference]) {
    references.sort_by_key(|r| {
        (
            // COMMENT: r.moved_cell.is_none(), // openmw 0.47 bug that requires MVRF records to be on top
            // COMMENT: accompany this change with the same change in tes3 library(libs/esp/src/types/cell.rs):
            // COMMENT: reference.moved_cell.is_none(), // openmw 0.47 bug that requires MVRF records to be on top
            !r.persistent(),
            match r.mast_index {
                0 => u32::MAX,
                i => i,
            },
            r.refr_index,
        )
    });
}

pub fn process_moved_instances(out: &mut Out, h: &mut Helper) -> Result<()> {
    if !h.g.r.moved_instances.is_empty() {
        for (id, grids) in &h.g.r.moved_instances {
            let old_cell_id = match h.g.r.ext_cells.get(&grids.old_grid) {
                None => {
                    return Err(anyhow!(
                        "Error: failed to find old_cell_id for moved instance"
                    ))
                }
                Some(cell_meta) => cell_meta.global_cell_id,
            };
            let new_cell_id = match h.g.r.ext_cells.get(&grids.new_grid) {
                None => {
                    return Err(anyhow!(
                        "Error: failed to find new_cell_id for moved instance"
                    ))
                }
                Some(cell_meta) => cell_meta.global_cell_id,
            };
            let reference = match out
                .cell
                .get_mut(old_cell_id)
                .with_context(|| {
                    format!("Bug: out.cell with old_cell_id = \"{old_cell_id}\" not found")
                })?
                .0
                .references
                .remove(id)
            {
                None => return Err(anyhow!("Error: failed to find moved instance in old cell")),
                Some(reference) => reference,
            };
            let reference_clone = reference.clone();
            if out
                .cell
                .get_mut(new_cell_id)
                .with_context(|| {
                    format!("Bug: out.cell with new_cell_id = \"{new_cell_id}\" not found")
                })?
                .0
                .references
                .insert(
                    *id,
                    Reference {
                        moved_cell: None,
                        ..reference
                    },
                )
                .is_some()
            {
                return Err(anyhow!("Error: new cell already had moved instance"));
            } else if h.g.list_options.turn_normal_grass {
                let old_ref_source = match h.g.r.ext_ref_sources.get_mut(&grids.old_grid) {
                    None => {
                        return Err(anyhow!(
                            "Bug: failed to find cell(old_grid) \"{:?}\" in in ext_ref_sources",
                            &grids.old_grid
                        ))
                    }
                    Some(ext_ref) => match ext_ref.0.remove(id) {
                        None => {
                            return Err(anyhow!(
                                "Bug: failed to properly delete reference \"{:?}\" from cell(old_grid) \"{:?}\" in in ext_ref_sources, it was missing",
                                id,
                                &grids.old_grid
                            ))
                        }
                        Some((old_ref_source, _, _)) => {
                            ext_ref.1.insert(*id, (old_ref_source, reference_clone));
                            old_ref_source
                        }
                    },
                };
                match h.g.r.ext_ref_sources.get_mut(&grids.new_grid) {
                    None => return Err(anyhow!("WTF moved ext_ref_souces")),
                    Some(ext_ref) => ext_ref.0.insert(*id, (old_ref_source, false, true)),
                }
            } else {
                None
            };
        }
    }
    Ok(())
}

pub fn show_ignored_ref_errors(
    ignored_ref_errors: &[IgnoredRefError],
    plugin_name: &PluginName,
    cell: bool,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    if !ignored_ref_errors.is_empty() {
        let ignored_ref_errors_len = ignored_ref_errors.len();
        let (
            mut master_suffix,
            mut cell_suffix,
            mut ref_suffix,
            mut encountered_prefix,
            mut encountered_suffix,
        ) = ("", "", "", "first ", "(check log for more)");
        if ignored_ref_errors_len > 1 {
            master_suffix = "s";
            cell_suffix = "s";
            ref_suffix = "s";
        } else {
            let ignored_ref_errors_first = ignored_ref_errors
                .first()
                .with_context(|| "Bug: ignored_ref_errors is empty")?;
            if ignored_ref_errors_first.cell_counter > 1 {
                cell_suffix = "s";
                ref_suffix = "s";
            } else if ignored_ref_errors_first.ref_counter > 1 {
                ref_suffix = "s";
            } else {
                encountered_prefix = "";
                encountered_suffix = "";
            }
        };
        let cell_msg_part = if cell {
            format!("for cell{cell_suffix} ")
        } else {
            String::new()
        };
        let mut text = format!(
            "Warning: probably outdated plugin \"{plugin_name}\" contains modified cell reference{ref_suffix} {cell_msg_part}missing from master{master_suffix}:"
        );
        for master in ignored_ref_errors {
            write!(text,
                "\n  Master \"{}\"({} cell{cell_suffix}, {} ref{ref_suffix}), {encountered_prefix}error encountered was{encountered_suffix}:\n{}",
                master.master, master.cell_counter, master.ref_counter, master.first_encounter,
            )?;
        }
        msg(text, 0, cfg, log)?;
    }
    Ok(())
}
