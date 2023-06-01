use crate::{
    msg, references_sorted, CellExtGrid, CellMeta, Cfg, Helper, IgnoredRefError, LocalMaster, LocalMergedMaster, Log,
    MergedPluginMeta, MergedPluginRefr, Mode, MovedInstanceGrids, MovedInstanceId, Out, RefrId, StatsUpdateKind,
};
use anyhow::{anyhow, Context, Result};
use hashbrown::{hash_map::Entry, HashMap};
use tes3::esp::{Cell, CellFlags, Reference};

pub(crate) fn process_cell(cell: Cell, out: &mut Out, name: &str, h: &mut Helper, cfg: &Cfg, log: &mut Log) -> Result<()> {
    let mut missing_cell_in_merged_master_shown = false;
    let mut missing_ref_in_merged_master_shown = false;
    let mut plugin_refrs: Vec<MergedPluginRefr> = Vec::new();
    let mut local_references: Vec<&Reference> = match h.g.mode {
        Mode::Grass => cell
            .references
            .values()
            .filter(|x| !cfg.guts.grass_filter.contains(&x.id.to_lowercase()))
            .collect(),

        _ => cell.references.values().collect(),
    };
    if let Mode::Grass = h.g.mode {
        h.l.stats.grass_filtered(cell.references.len() - local_references.len());
    }
    references_sorted(&mut local_references);
    macro_rules! int_or_ext_cell {
        ($name:expr, $key:ident, $grid:expr) => {
            match $name.entry($key) {
                Entry::Vacant(v) => {
                    let mut references: HashMap<(u32, u32), Reference> = HashMap::new();
                    for local_reference in local_references {
                        if local_reference.mast_index == 0 {
                            h.g.refr += 1;
                            add_simple_reference(local_reference, &mut references, &mut plugin_refrs, h.g.refr);
                            if !h.g.contains_non_external_refs {
                                h.g.contains_non_external_refs = true;
                            }
                        } else {
                            match h.l.merged_masters.iter().find(|x| x.local_id == local_reference.mast_index) {
                                Some(merged_master) => {
                                    let text = missing_ref_text(&cell, &merged_master.name_low, 0);
                                    if h.g.no_ignore_errors {
                                        return Err(anyhow!("Merged {}", text));
                                    } else {
                                        missing_ref_append(text, &mut h.l.ignored_cell_errors, &merged_master, &mut missing_cell_in_merged_master_shown, cfg, log)?;
                                        continue;
                                    }
                                }
                                None => {
                                    if h.g.strip_masters {
                                        let text = format!("Output plugin \"{}\": masters will not be stripped due to encountering external reference", name);
                                        msg(text, 0, cfg, log)?;
                                        h.g.strip_masters = false;
                                    }
                                    add_external_reference(local_reference, &mut references, &h.l.masters)
                                        .with_context(|| format!("Failed to add vacant external reference"))?
                                }
                            }
                        }
                    }
                    let cell_len = out.cell.len();
                    v.insert(CellMeta {
                        global_cell_id: cell_len,
                        plugin_metas: vec![MergedPluginMeta {
                            plugin_name_low: h.l.plugin_name_low.clone(),
                            plugin_refrs,
                        }],
                    });
                    out.cell.push((
                        Cell {
                            references,
                            ..cell.clone()
                        },
                        Vec::new(),
                    ));
                    h.l.stats.cell(StatsUpdateKind::Processed);
                }
                Entry::Occupied(o) => {
                    let o_cell = &mut out.cell[o.get().global_cell_id];
                    if o_cell.0.flags != cell.flags
                        || o_cell.0.name != cell.name
                        || o_cell.0.data != cell.data
                        || o_cell.0.region != cell.region
                        || (cell.map_color.is_some() && o_cell.0.map_color != cell.map_color)
                        || (cell.water_height.is_some() && o_cell.0.water_height != cell.water_height)
                        || (cell.atmosphere_data.is_some() && o_cell.0.atmosphere_data != cell.atmosphere_data)
                    {
                        if o_cell.1.is_empty() {
                            o_cell.1.push(Cell {
                                references: HashMap::new(),
                                ..o_cell.0.clone()
                            });
                        }
                        o_cell.1.push(Cell {
                            references: HashMap::new(),
                            ..cell.clone()
                        });
                        if o_cell.0.flags != cell.flags {
                            o_cell.0.flags = cell.flags;
                        }
                        if o_cell.0.name != cell.name {
                            o_cell.0.name = cell.name;
                        }
                        if o_cell.0.data != cell.data {
                            o_cell.0.data = cell.data;
                        }
                        if o_cell.0.region != cell.region {
                            o_cell.0.region = cell.region;
                        }
                        if cell.map_color.is_some() && o_cell.0.map_color != cell.map_color {
                            o_cell.0.map_color = cell.map_color;
                        }
                        if cell.water_height.is_some() && o_cell.0.water_height != cell.water_height {
                            o_cell.0.water_height = cell.water_height;
                        }
                        if cell.map_color.is_some() && o_cell.0.atmosphere_data != cell.atmosphere_data {
                            o_cell.0.atmosphere_data = cell.atmosphere_data;
                        }
                    }

                    for local_reference in local_references {
                        if local_reference.mast_index == 0 {
                            h.g.refr += 1;
                            add_simple_reference(local_reference, &mut o_cell.0.references, &mut plugin_refrs, h.g.refr);
                            if !h.g.contains_non_external_refs {
                                h.g.contains_non_external_refs = true;
                            }
                        } else {
                            match h.l.merged_masters.iter().find(|x| x.local_id == local_reference.mast_index) {
                                Some(local_merged_master) => {
                                    match get_global_refr(local_reference, local_merged_master, &o.get().plugin_metas) {
                                        Ok(refr_index) => modify_global_reference(
                                            local_reference,
                                            &mut o_cell.0.references,
                                            refr_index,
                                            &mut h.g.r.moved_instances,
                                            $grid,
                                        )
                                        .with_context(|| format!("Failed to modify global reference"))?,
                                        Err(err) => {
                                            let text = missing_ref_text(&o_cell.0, &local_merged_master.name_low, local_reference.refr_index);
                                            if h.g.no_ignore_errors {
                                                return Err(anyhow!("Merged {}\n{}", text, err));
                                            } else {
                                                missing_ref_append(text, &mut h.l.ignored_ref_errors, &local_merged_master, &mut missing_ref_in_merged_master_shown, cfg, log)?;
                                                continue;
                                            }
                                        }
                                    };
                                }
                                None => {
                                    add_external_reference(local_reference, &mut o_cell.0.references, &h.l.masters)
                                        .with_context(|| format!("Failed to add occupied external reference"))?;
                                }
                            }
                        }
                    }
                    o.into_mut().plugin_metas.push(MergedPluginMeta {
                        plugin_name_low: h.l.plugin_name_low.clone(),
                        plugin_refrs,
                    });
                    h.l.stats.cell(StatsUpdateKind::Merged);
                }
            }
        };
    }
    if cell.data.flags.contains(CellFlags::IS_INTERIOR) {
        let cell_name_low = cell.name.to_lowercase();
        int_or_ext_cell!(h.g.r.int_cells, cell_name_low, None);
    } else {
        let cell_data_grid = cell.data.grid;
        int_or_ext_cell!(h.g.r.ext_cells, cell_data_grid, Some(cell_data_grid));
    }
    Ok(())
}

fn add_simple_reference(
    local_reference: &Reference,
    references: &mut HashMap<(u32, u32), Reference>,
    local_plugin_refrs: &mut Vec<MergedPluginRefr>,
    refr: RefrId,
) {
    let new_reference = Reference {
        refr_index: refr,
        object_count: if local_reference.object_count == Some(1) {
            None
        } else {
            local_reference.object_count
        },
        scale: if local_reference.scale == Some(1.0) {
            None
        } else {
            local_reference.scale
        },
        ..local_reference.clone()
    };
    references.insert((0, refr), new_reference);
    local_plugin_refrs.push(MergedPluginRefr {
        local_refr: local_reference.refr_index,
        global_refr: refr,
    });
}

fn add_external_reference(
    local_reference: &Reference,
    references: &mut HashMap<(u32, u32), Reference>,
    local_masters: &[LocalMaster],
) -> Result<()> {
    let mast_index = match local_masters.iter().find(|x| x.local_id == local_reference.mast_index) {
        Some(local_master) => local_master.global_id,
        None => {
            return Err(anyhow!(
                "Failed to find local master id for reference \"{}\" with master index \"{}\"",
                local_reference.refr_index,
                local_reference.mast_index
            ))
        }
    };
    let new_reference = Reference {
        mast_index,
        object_count: if local_reference.object_count == Some(1) {
            None
        } else {
            local_reference.object_count
        },
        scale: if local_reference.scale == Some(1.0) {
            None
        } else {
            local_reference.scale
        },
        moved_cell: if local_reference.deleted.is_some() {
            None
        } else {
            local_reference.moved_cell
        },
        ..local_reference.clone()
    };
    references.insert((mast_index, local_reference.refr_index), new_reference);
    Ok(())
}

fn get_global_refr(
    local_reference: &Reference,
    local_merged_master: &LocalMergedMaster,
    plugin_metas: &[MergedPluginMeta],
) -> Result<RefrId> {
    let refr_index = match plugin_metas.iter().find(|x| x.plugin_name_low == *local_merged_master.name_low) {
        None => {
            return Err(anyhow!(
                "Failed to find any references added by master \"{}\"",
                local_merged_master.name_low
            ))
        }
        Some(merged_plugin_meta) => {
            match merged_plugin_meta
                .plugin_refrs
                .iter()
                .find(|x| x.local_refr == local_reference.refr_index)
            {
                None => {
                    return Err(anyhow!(
                        "Failed to find reference \"{}\" that should've been in master \"{}\"",
                        local_reference.refr_index,
                        local_merged_master.name_low
                    ))
                }
                Some(merged_plugin_refr) => merged_plugin_refr.global_refr,
            }
        }
    };
    Ok(refr_index)
}

fn modify_global_reference(
    local_reference: &Reference,
    references: &mut HashMap<(u32, u32), Reference>,
    refr_index: RefrId,
    moved_instances: &mut HashMap<MovedInstanceId, MovedInstanceGrids>,
    grid: Option<CellExtGrid>,
) -> Result<()> {
    match references.entry((0, refr_index)) {
        Entry::Vacant(_) => {
            return Err(anyhow!(
                "Error: there is no already merged reference with refr_index \"{}\"",
                refr_index
            ));
        }
        Entry::Occupied(mut o) => {
            let value = o.get_mut();
            match local_reference.moved_cell {
                None => {
                    moved_instances.remove(&(0, refr_index));
                }
                Some(new_grid) => {
                    let old_grid = match grid {
                        None => return Err(anyhow!("Error: interior cell should not contain moved records")),
                        Some(old_grid) => old_grid,
                    };
                    match moved_instances.entry((0, refr_index)) {
                        Entry::Vacant(v) => {
                            v.insert(MovedInstanceGrids { old_grid, new_grid });
                        }
                        Entry::Occupied(mut o) => {
                            let moved_instance_value = o.get_mut();
                            *moved_instance_value = MovedInstanceGrids { old_grid, new_grid };
                        }
                    }
                }
            }
            *value = Reference {
                mast_index: 0,
                refr_index,
                object_count: if local_reference.object_count == Some(1) {
                    None
                } else {
                    local_reference.object_count
                },
                scale: if local_reference.scale == Some(1.0) {
                    None
                } else {
                    local_reference.scale
                },
                ..local_reference.clone()
            };
        }
    };
    Ok(())
}

fn missing_ref_text(cell: &Cell, master_name_low: &String, refr_index: u32) -> String {
    let cell_name = if cell.data.flags.contains(CellFlags::IS_INTERIOR) {
        cell.name.clone()
    } else {
        format!("({}, {})", cell.data.grid.0, cell.data.grid.1)
    };
    format!(
        "master \"{}\" doesn't contain {}cell \"{}\"",
        master_name_low,
        if refr_index == 0 {
            String::new()
        } else {
            format!("reference \"{}\" in ", refr_index)
        },
        cell_name
    )
}

fn missing_ref_append(
    text: String,
    ignored_ref_errors: &mut Vec<IgnoredRefError>,
    merged_master: &LocalMergedMaster,
    flag: &mut bool,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let text = format!("    Ignored error: merged {}", text);
    match ignored_ref_errors.iter_mut().find(|x| x.master == merged_master.name_low) {
        Some(ignored_ref_error) => {
            if !*flag {
                ignored_ref_error.cell_counter += 1;
                msg(&text, 2, cfg, log)?;
                *flag = true;
            } else if cfg.show_all_missing_refs {
                msg(&text, 2, cfg, log)?;
            }
            ignored_ref_error.ref_counter += 1;
        }
        None => {
            msg(&text, 2, cfg, log)?;
            *flag = true;
            ignored_ref_errors.push(IgnoredRefError {
                master: merged_master.name_low.clone(),
                first_encounter: text,
                cell_counter: 1,
                ref_counter: 1,
            });
        }
    };
    Ok(())
}
