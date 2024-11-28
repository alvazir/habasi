use crate::{
    increment, msg, references_sorted, CellExtGrid, CellMeta, Cfg, Helper, IgnoredRefError,
    ListOptions, LocalMaster, LocalMergedMaster, Log, MastId, MergedPluginMeta, MergedPluginRefr,
    Mode, MovedInstanceGrids, MovedInstanceId, OldRefSources, Out, RefSources, RefrId,
    StatsUpdateKind,
};
use anyhow::{anyhow, Context as _, Result};
use hashbrown::{hash_map::Entry, HashMap};
use std::fmt::Write as _;
use tes3::esp::{Cell, CellFlags, Reference};

#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
pub fn process(
    cell: Cell,
    out: &mut Out,
    name: &str,
    h: &mut Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let plugin_num = MastId::try_from(h.g.plugins_processed.len()).with_context(|| {
        format!(
            "Bug: failed to cast {:?}(plugins_processed.len(), usize) to u32(MastId)",
            h.g.plugins_processed.len()
        )
    })?;
    let mut missing_cell_in_merged_master_shown = false;
    let mut missing_ref_in_merged_master_shown = false;
    let mut plugin_refrs: Vec<MergedPluginRefr> = Vec::new();
    #[allow(clippy::wildcard_enum_match_arm)]
    let mut local_references: Vec<&Reference> = match h.g.list_options.mode {
        Mode::Grass => cell
            .references
            .values()
            .filter(|x| !cfg.advanced.grass_filter.contains(&x.id.to_lowercase()))
            .collect(),

        _ => cell.references.values().collect(),
    };
    h.l.stats
        .instances_processed_add_count(cell.references.len())?;
    if matches!(h.g.list_options.mode, Mode::Grass) {
        h.l.stats
            .grass_filtered(cell.references.len().checked_sub(local_references.len()).with_context(|| {
                format!(
                    "Bug: overflow subtracting local_references.len() = \"{}\" from cell.references.len() = \"{}\"",
                    local_references.len(),
                    cell.references.len()
                )
            })?)?;
    }
    references_sorted(&mut local_references);
    macro_rules! int_or_ext_cell {
        ($name:expr, $key:ident, $grid:expr) => {
            match $name.entry($key) {
                Entry::Vacant(v) => {
                    let mut references: HashMap<(MastId, RefrId), Reference> = HashMap::new();
                    let mut ref_sources = (HashMap::new(), HashMap::new());
                    for local_reference in local_references {
                        if local_reference.mast_index == 0 {
                            if h.g.refr < u32::MAX {
                                h.g.refr = increment!(h.g.refr);
                            } else {
                                return Err(anyhow!("Error: limit of {} references per plugin reached. Split the list into smaller parts.", u32::MAX));
                            }
                            add_simple_reference(local_reference, &mut references, &mut plugin_refrs, h.g.refr, plugin_num, &mut ref_sources, h.g.list_options.turn_normal_grass);
                            if !h.g.contains_non_external_refs {
                                h.g.contains_non_external_refs = true;
                            }
                        } else {
                            match h.l.merged_masters.iter().find(|x| x.local_id == local_reference.mast_index) {
                                Some(merged_master) => {
                                    missing_ref_text(
                                        &mut h.t.missing_ref_text,
                                        h.g.list_options.no_ignore_errors,
                                        &cell,
                                        &merged_master.name_low,
                                        0
                                    )?;
                                    if h.g.list_options.no_ignore_errors {
                                        return Err(anyhow!("{}", h.t.missing_ref_text));
                                    }
                                    missing_ref_append(
                                        &h.t.missing_ref_text,
                                        &mut h.l.ignored_cell_errors,
                                        &merged_master,
                                        &mut missing_cell_in_merged_master_shown,
                                        &h.g.list_options,
                                        cfg,
                                        log
                                    )?;
                                }
                                None => {
                                    if h.g.list_options.strip_masters {
                                        let text = format!("Output plugin \"{name}\": masters will not be stripped due to encountering external reference");
                                        msg(text, 0, cfg, log)?;
                                        h.g.list_options.strip_masters = false;
                                    }
                                    add_external_reference(local_reference, &mut references, &h.l.masters, &mut ref_sources, h.g.list_options.turn_normal_grass)
                                        .with_context(|| format!("Failed to add vacant external reference"))?
                                }
                            }
                        }
                    }
                    let cell_len = out.cell.len();
                    v.insert(CellMeta {
                        global_cell_id: cell_len,
                        plugin_metas: vec![MergedPluginMeta {
                            plugin_name_low: h.l.plugin_info.name_low.clone(),
                            plugin_refrs,
                        }],
                    });
                    if h.g.list_options.turn_normal_grass {
                        if let Some(grid) = $grid {
                            h.g.r.ext_ref_sources.insert(grid, ref_sources);
                        }
                    }
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
                    let o_cell = out.cell.get_mut(o.get().global_cell_id)
                        .with_context(|| format!("Bug: indexing slicing out.cell[{}]", o.get().global_cell_id))?;
                    let mut dummy_source = (HashMap::new(), HashMap::new());
                    let mut ref_sources = match h.g.list_options.turn_normal_grass {
                        false => &mut dummy_source,
                        true => match $grid {
                            None => &mut dummy_source,
                            Some(grid) => match h.g.r.ext_ref_sources.get_mut(&grid) {
                                None => return Err(anyhow!("Bug: failed to find cell \"{:?}\" in in ext_ref_sources", &grid)),
                                Some(v) => v,
                            },
                        },
                    };
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
                        if cell.atmosphere_data.is_some() && o_cell.0.atmosphere_data != cell.atmosphere_data {
                            o_cell.0.atmosphere_data = cell.atmosphere_data;
                        }
                    } else if h.g.list_options.debug {
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
                    } else { //
                    }

                    for local_reference in local_references {
                        if local_reference.mast_index == 0 {
                            if h.g.refr < u32::MAX {
                                h.g.refr = increment!(h.g.refr);
                            } else {
                                return Err(anyhow!("Error: limit of {} references per plugin reached. Split the list into smaller parts.", u32::MAX));
                            }
                            add_simple_reference(local_reference, &mut o_cell.0.references, &mut plugin_refrs, h.g.refr, plugin_num, ref_sources, h.g.list_options.turn_normal_grass);
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
                                            missing_ref_text(
                                                &mut h.t.missing_ref_text,
                                                h.g.list_options.no_ignore_errors,
                                                &o_cell.0,
                                                &local_merged_master.name_low,
                                                local_reference.refr_index
                                            )?;
                                            if h.g.list_options.no_ignore_errors {
                                                return Err(anyhow!("{}\n{err:#}", h.t.missing_ref_text));
                                            }
                                            missing_ref_append(
                                                &h.t.missing_ref_text,
                                                &mut h.l.ignored_ref_errors,
                                                &local_merged_master,
                                                &mut missing_ref_in_merged_master_shown,
                                                &h.g.list_options,
                                                cfg,
                                                log
                                            )?;
                                        }
                                    };
                                }
                                None => {
                                    add_external_reference(local_reference, &mut o_cell.0.references, &h.l.masters, &mut ref_sources, h.g.list_options.turn_normal_grass)
                                        .with_context(|| format!("Failed to add occupied external reference"))?;
                                }
                            }
                        }
                    }
                    o.into_mut().plugin_metas.push(MergedPluginMeta {
                        plugin_name_low: h.l.plugin_info.name_low.clone(),
                        plugin_refrs,
                    });
                    h.l.stats.cell(StatsUpdateKind::Merged);
                }
            }
        };
    }
    if cell.data.flags.contains(CellFlags::IS_INTERIOR) {
        let cell_name_low = cell.name.to_lowercase();
        int_or_ext_cell!(h.g.r.int_cells, cell_name_low, None::<(i32, i32)>);
    } else {
        let cell_data_grid = cell.data.grid;
        int_or_ext_cell!(h.g.r.ext_cells, cell_data_grid, Some(cell_data_grid));
    }
    Ok(())
}

fn add_simple_reference(
    local_reference: &Reference,
    references: &mut HashMap<(MastId, RefrId), Reference>,
    local_plugin_refrs: &mut Vec<MergedPluginRefr>,
    refr: RefrId,
    plugin_num: MastId,
    ref_sources: &mut (RefSources, OldRefSources),
    turn_normal_grass: bool,
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
        temporary: if local_reference.destination.is_some() {
            false
        } else {
            local_reference.temporary
        },
        ..local_reference.clone()
    };
    references.insert((0, refr), new_reference);
    if turn_normal_grass {
        ref_sources.0.insert(
            (0, refr),
            ((plugin_num, local_reference.refr_index), false, false),
        );
    }
    local_plugin_refrs.push(MergedPluginRefr {
        local_refr: local_reference.refr_index,
        global_refr: refr,
    });
}

fn add_external_reference(
    local_reference: &Reference,
    references: &mut HashMap<(u32, u32), Reference>,
    local_masters: &[LocalMaster],
    ref_sources: &mut (RefSources, OldRefSources),
    turn_normal_grass: bool,
) -> Result<()> {
    let mast_index = match local_masters
        .iter()
        .find(|x| x.local_id == local_reference.mast_index)
    {
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
        scale: if local_reference.scale == Some(1.0) || local_reference.deleted.is_some() {
            None
        } else {
            local_reference.scale
        },
        temporary: if local_reference.destination.is_some() {
            false
        } else {
            local_reference.temporary
        },
        moved_cell: if local_reference.deleted.is_some() {
            None
        } else {
            local_reference.moved_cell
        },
        ..local_reference.clone()
    };
    references.insert((mast_index, local_reference.refr_index), new_reference);
    if turn_normal_grass {
        ref_sources.0.insert(
            (mast_index, local_reference.refr_index),
            ((mast_index, local_reference.refr_index), true, false),
        );
    }
    Ok(())
}

fn get_global_refr(
    local_reference: &Reference,
    local_merged_master: &LocalMergedMaster,
    plugin_metas: &[MergedPluginMeta],
) -> Result<RefrId> {
    let refr_index = match plugin_metas
        .iter()
        .find(|x| x.plugin_name_low == *local_merged_master.name_low)
    {
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
    references: &mut HashMap<(MastId, RefrId), Reference>,
    refr_index: RefrId,
    moved_instances: &mut HashMap<MovedInstanceId, MovedInstanceGrids>,
    grid: Option<CellExtGrid>,
) -> Result<()> {
    match references.entry((0, refr_index)) {
        Entry::Vacant(_) => Err(anyhow!(
            "Error: there is no already merged reference with refr_index \"{refr_index}\"",
        )),
        Entry::Occupied(mut o) => {
            if let Some(new_grid) = local_reference.moved_cell {
                let Some(old_grid) = grid else {
                    return Err(anyhow!(
                        "Error: interior cell should not contain moved records"
                    ));
                };
                moved_instances.insert((0, refr_index), MovedInstanceGrids { old_grid, new_grid });
            } else {
                moved_instances.remove(&(0, refr_index));
            }
            *o.get_mut() = Reference {
                mast_index: 0,
                refr_index,
                object_count: if local_reference.object_count == Some(1) {
                    None
                } else {
                    local_reference.object_count
                },
                scale: if local_reference.scale == Some(1.0) || local_reference.deleted.is_some() {
                    None
                } else {
                    local_reference.scale
                },
                temporary: if local_reference.destination.is_some() {
                    false
                } else {
                    local_reference.temporary
                },
                ..local_reference.clone()
            };
            Ok(())
        }
    }
}

fn missing_ref_text(
    text: &mut String,
    is_error: bool,
    cell: &Cell,
    master_name_low: &String,
    refr_index: u32,
) -> Result<()> {
    text.clear();
    write!(
        text,
        "{} master \"{master_name_low}\" doesn't contain ",
        if is_error {
            "Merged"
        } else {
            "    Ignored error: merged"
        }
    )?;
    if refr_index != 0 {
        write!(text, "reference \"{refr_index}\" in ")?;
    }
    if cell.data.flags.contains(CellFlags::IS_INTERIOR) {
        write!(text, "cell \"{}\"", cell.name)?;
    } else {
        write!(
            text,
            "cell \"({}, {})\"",
            cell.data.grid.0, cell.data.grid.1
        )?;
    }
    Ok(())
}

fn missing_ref_append(
    text: &str,
    ignored_ref_errors: &mut Vec<IgnoredRefError>,
    merged_master: &LocalMergedMaster,
    flag: &mut bool,
    list_options: &ListOptions,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    if let Some(ignored_ref_error) = ignored_ref_errors
        .iter_mut()
        .find(|x| x.master == merged_master.name_low)
    {
        if !*flag {
            ignored_ref_error.cell_counter = increment!(ignored_ref_error.cell_counter);
            if !list_options.no_show_missing_refs {
                msg(text, 2, cfg, log)?;
            }
            *flag = true;
        } else if !list_options.no_show_missing_refs && list_options.show_all_missing_refs {
            msg(text, 2, cfg, log)?;
        } else { //
        }
        ignored_ref_error.ref_counter = increment!(ignored_ref_error.ref_counter);
    } else {
        if !list_options.no_show_missing_refs {
            msg(text, 2, cfg, log)?;
        }
        *flag = true;
        ignored_ref_errors.push(IgnoredRefError {
            master: merged_master.name_low.clone(),
            first_encounter: text.to_owned(),
            cell_counter: 1,
            ref_counter: 1,
        });
    }
    Ok(())
}
