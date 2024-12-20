use super::make_header;
use crate::{
    get_tng_dir_and_plugin_names, increment, load_order, make_tng_meshes, msg, msg_no_log,
    references_sorted, CellExtGrid, Cfg, HeaderText, Helper, Log, MastId, OldRefSources, Out,
    PluginInfo, RefSources, RefrId, TurnNormalGrass,
};
use anyhow::{anyhow, Context as _, Result};
use hashbrown::{HashMap, HashSet};
use rayon::iter::{
    IntoParallelIterator as _, IntoParallelRefIterator as _, IntoParallelRefMutIterator as _, ParallelIterator as _,
};
use std::fmt::Write as _;
use tes3::esp::{Cell, CellFlags, ObjectFlags, Plugin, Reference, Static, TES3Object};

type FoundStatIdsVec = Vec<String>;
type FoundStatIds = HashSet<String>;
type NewMasterIds = Vec<u32>;
type NumCells = Vec<(usize, Cell)>;
type NumCellsRefCounted = Vec<(usize, u32, Cell)>;
type DelRefCellHelper = (Cell, NewMasterIds, FoundStatIdsVec);
type DelRefCells = Vec<Cell>;
type GrassCells = Vec<Cell>;
type NewMasters = Vec<(String, u64)>;
type MasterRemapTable = HashMap<u32, u32>;
type TngCellHelper = (DelRefCells, GrassCells, NewMasters, FoundStatIds);

macro_rules! check_push {
    ($ids:ident, $id:expr) => {
        if !$ids.contains(&$id) {
            $ids.push($id);
        }
    };
}

pub fn make_turn_normal_grass(
    name: &str,
    out: &mut Out,
    h: &mut Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<(String, Plugin, String, Plugin)> {
    load_order::scan(h, cfg, log)
        .with_context(|| "Failed to scan load order while trying to turn normal grass")?;
    let (del_ref_cells, grass_cells, new_masters, found_stat_ids) = get_tng_cells(
        &mut out.cell,
        &h.g.r.ext_ref_sources,
        &out.masters,
        &h.g.plugins_processed,
        h.g.list_options.exclude_deleted_records,
        cfg,
        log,
    )
    .with_context(|| "Failed to process cells while trying to turn normal grass")?;
    h.g.found_stat_ids = found_stat_ids;
    let (dir, plugin_deleted_content_name, plugin_grass_name) =
        get_tng_dir_and_plugin_names(name, cfg)
            .with_context(|| "Failed to get turn normal grass directory or plugin names")?;
    make_tng_meshes(dir, out, h, cfg, log)?;
    let tng_statics = make_tng_statics(&plugin_grass_name, h, cfg, log)
        .with_context(|| "Failed to process STAT records while trying to turn normal grass")?;
    let author = format!(
        "{}{}",
        &cfg.guts.header_author, &cfg.guts.turn_normal_grass_header_author_append
    );
    let mut description = cfg
        .guts
        .turn_normal_grass_header_description_content
        .join("\n");
    let plugin_deleted_content = make_tng_plugin(
        &plugin_deleted_content_name,
        del_ref_cells,
        None,
        new_masters.clone(),
        HeaderText::new(&author, &description, cfg, log)?,
        h,
        cfg,
        log,
    )
    .with_context(|| format!("Failed to make plugin \"{}\"", &plugin_deleted_content_name))?;
    description = cfg
        .guts
        .turn_normal_grass_header_description_groundcover
        .join("\n");
    let plugin_grass = make_tng_plugin(
        &plugin_grass_name,
        grass_cells,
        Some(tng_statics),
        new_masters,
        HeaderText::new(&author, &description, cfg, log)?,
        h,
        cfg,
        log,
    )
    .with_context(|| format!("Failed to make plugin \"{}\"", &plugin_grass_name))?;
    Ok((
        plugin_deleted_content_name,
        plugin_deleted_content,
        plugin_grass_name,
        plugin_grass,
    ))
}

fn get_tng_cells(
    cells: &mut [(Cell, Vec<Cell>)],
    ext_ref_sources: &HashMap<CellExtGrid, (RefSources, OldRefSources)>,
    masters: &[(String, u64)],
    plugins_processed: &[PluginInfo],
    exclude_deleted_records: bool,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<TngCellHelper> {
    let masters_len = u32::try_from(masters.len()).with_context(|| {
        format!(
            "Bug: failed to cast {:?}(masters.len(), usize) to u32",
            masters.len()
        )
    })?;
    let (del_ref_num_cells, new_master_ids, found_stat_ids) = make_del_ref_num_cells(
        cells,
        ext_ref_sources,
        masters_len,
        exclude_deleted_records,
        cfg,
    )
    .with_context(|| "Failed to process cells to find potential grass")?;
    let master_remap_table = make_master_remap_table(&new_master_ids, masters_len)
        .with_context(|| "Failed to produce master remap table, possibly due to a bug")?;
    let new_masters = make_new_masters(
        masters,
        &new_master_ids,
        masters_len,
        &master_remap_table,
        plugins_processed,
        cfg,
        log,
    )
    .with_context(|| "Failed to produce new masters list")?;
    let del_ref_master_renum_num_cells =
        make_del_ref_master_renum_num_cells(del_ref_num_cells, &master_remap_table)
            .with_context(|| "Failed to make_del_ref_master_renum_num_cells")?;
    let grass_cells = make_grass_cells(&del_ref_master_renum_num_cells)
        .with_context(|| "Failed to make_grass_cells")?;
    let del_ref_cells = make_del_ref_cells(del_ref_master_renum_num_cells);
    Ok((del_ref_cells, grass_cells, new_masters, found_stat_ids))
}

fn make_del_ref_num_cells(
    cells: &mut [(Cell, Vec<Cell>)],
    ext_ref_sources: &HashMap<CellExtGrid, (RefSources, OldRefSources)>,
    masters_len: u32,
    exclude_deleted_records: bool,
    cfg: &Cfg,
) -> Result<(NumCells, NewMasterIds, FoundStatIds)> {
    let mut del_ref_num_cells: NumCells = Vec::new();
    let mut new_master_ids: NewMasterIds = Vec::new();
    let mut found_stat_ids: FoundStatIds = HashSet::new();
    let del_ref_num_cells_helper: Vec<(usize, Option<DelRefCellHelper>)> =
        make_del_ref_num_cells_helper(
            cells,
            ext_ref_sources,
            masters_len,
            exclude_deleted_records,
            cfg,
        )
        .with_context(|| "Failed to produce deleted grass references, possibly due to a bug")?;
    for (id, del_ref_cell_helper) in del_ref_num_cells_helper {
        if let Some((cell, raw_new_master_ids, raw_found_stat_ids)) = del_ref_cell_helper {
            del_ref_num_cells.push((id, cell));
            for master_id in raw_new_master_ids {
                check_push!(new_master_ids, master_id);
            }
            for stat_id in raw_found_stat_ids {
                if !found_stat_ids.contains(&stat_id) {
                    found_stat_ids.insert(stat_id);
                }
            }
        }
    }
    new_master_ids.sort_unstable();
    Ok((del_ref_num_cells, new_master_ids, found_stat_ids))
}

#[allow(clippy::too_many_lines)]
fn make_del_ref_num_cells_helper(
    cells: &mut [(Cell, Vec<Cell>)],
    ext_ref_sources: &HashMap<CellExtGrid, (RefSources, OldRefSources)>,
    masters_len: u32,
    exclude_deleted_records: bool,
    cfg: &Cfg,
) -> Result<Vec<(usize, Option<DelRefCellHelper>)>> {
    let mut del_ref_cell_helper = cells
        .iter_mut()
        .enumerate()
        .collect::<Vec<_>>()
        .par_iter_mut()
        .map(|&mut (id, &mut (ref mut cell, _))| -> Result<(usize, Option<DelRefCellHelper>)> {
            if !(cell.data.flags.contains(CellFlags::IS_INTERIOR)
                || (exclude_deleted_records && cell.flags.contains(ObjectFlags::DELETED)))
            {
                let mut new_master_ids: NewMasterIds = Vec::new();
                let mut found_stat_ids: FoundStatIdsVec = Vec::new();
                let mut deleted_references: HashMap<(MastId, RefrId), Reference> = HashMap::new();
                let mut list_of_refs_to_remove_full: Vec<(MastId, RefrId)> = Vec::new();
                macro_rules! del_ref_insert {
                    ($reference:ident, $mast_id:expr, $refr_id:expr) => {
                        deleted_references.insert(
                            ($mast_id, $refr_id),
                            Reference {
                                mast_index: $mast_id,
                                refr_index: $refr_id,
                                deleted: Some(true),
                                ..$reference.clone()
                            },
                        );
                    };
                }
                if !cell.references.is_empty() {
                    let ext_ref = match ext_ref_sources.get(&cell.data.grid) {
                        None => return Err(anyhow!("Bug: failed to find cell \"{:?}\" in ext_ref_sources", cell.data.grid)),
                        Some(ext_ref) => ext_ref,
                    };
                    for (key, reference) in &cell.references {
                        if reference.deleted.is_none()
                            && cfg.advanced.turn_normal_grass_stat_ids.set.contains(&reference.id.to_lowercase())
                        {
                            match ext_ref.0.get(&(reference.mast_index, reference.refr_index)) {
                                None => {
                                    return Err(anyhow!(
                                        "Bug: failed to find reference \"{:?}\" in cell \"{:?}\" in ext_ref_sources",
                                        key,
                                        cell.data.grid
                                    ))
                                }
                                Some(&(old_ids, is_external, is_moved)) => {
                                    if is_external {
                                        del_ref_insert!(reference, reference.mast_index, reference.refr_index);
                                        check_push!(found_stat_ids, reference.id.to_lowercase());
                                    } else if !is_moved {
                                        del_ref_insert!(
                                            reference,
                                            old_ids.0.checked_add(masters_len).with_context(|| format!(
                                                "Bug: overflow adding master_index = \"{}\" to masters_len= \"{masters_len}\"",
                                                old_ids.0
                                            ))?,
                                            old_ids.1
                                        );
                                        check_push!(new_master_ids, old_ids.0);
                                        check_push!(found_stat_ids, reference.id.to_lowercase());
                                        if exclude_deleted_records {
                                            list_of_refs_to_remove_full.push(*key);
                                        }
                                    } else if exclude_deleted_records && is_moved {
                                        list_of_refs_to_remove_full.push(*key);
                                    } else { //
                                    };
                                }
                            }
                        }
                    }
                }
                if let Some(ext_ref) = ext_ref_sources.get(&cell.data.grid) {
                    if !ext_ref.1.is_empty() {
                        for &(old_ids, ref reference) in ext_ref.1.values() {
                            if reference.deleted.is_none()
                                && cfg.advanced.turn_normal_grass_stat_ids.set.contains(&reference.id.to_lowercase())
                            {
                                del_ref_insert!(
                                    reference,
                                    old_ids.0.checked_add(masters_len).with_context(|| format!(
                                        "Bug: overflow adding master_index = \"{}\" to masters_len= \"{masters_len}\"",
                                        old_ids.0
                                    ))?,
                                    old_ids.1
                                );
                                check_push!(new_master_ids, old_ids.0);
                                check_push!(found_stat_ids, reference.id.to_lowercase());
                            }
                        }
                    }
                };
                if exclude_deleted_records && !list_of_refs_to_remove_full.is_empty() {
                    for i in list_of_refs_to_remove_full {
                        cell.references.remove(&i);
                    }
                }
                if !deleted_references.is_empty() {
                    return Ok((
                        id,
                        Some((
                            Cell {
                                references: deleted_references,
                                ..cell.clone()
                            },
                            new_master_ids,
                            found_stat_ids,
                        )),
                    ));
                }
            }
            Ok((id, None))
        })
        .collect::<Result<Vec<_>>>()?;
    del_ref_cell_helper.sort_by_key(|&(x, _)| x);
    Ok(del_ref_cell_helper)
}

fn make_master_remap_table(
    new_master_ids: &NewMasterIds,
    masters_len: u32,
) -> Result<MasterRemapTable> {
    let mut master_remap_table = HashMap::new();
    for (id, plugin_id) in new_master_ids.iter().enumerate() {
        let mut remap = u32::try_from(id)
            .with_context(|| format!("Bug: failed to cast {id:?}(id, usize) to u32"))?
            .checked_add(masters_len)
            .with_context(|| {
                format!("Bug: overflow adding id = \"{id}\" to masters_len = \"{masters_len}\"")
            })?;
        remap = increment!(remap);
        if master_remap_table
            .insert(
                plugin_id
                    .checked_add(masters_len)
                    .with_context(|| format!("Bug: overflow adding plugin_id = \"{plugin_id}\" to masters_len = \"{masters_len}\""))?,
                remap,
            )
            .is_some()
        {
            #[allow(clippy::arithmetic_side_effects)]
            return Err(anyhow!("Bug: doubled mapping in master_remap_table, it already had mapping for key \"{0}\" when tried to add mapping \"({0}, {1})\"", plugin_id + masters_len, remap));
        };
    }
    Ok(master_remap_table)
}

fn make_new_masters(
    masters: &[(String, u64)],
    new_master_ids: &NewMasterIds,
    masters_len: u32,
    remap_table: &MasterRemapTable,
    plugins_processed: &[PluginInfo],
    cfg: &Cfg,
    log: &mut Log,
) -> Result<Vec<(String, u64)>> {
    let mut new_masters: Vec<(String, u64)> = masters
        .iter()
        .map(|&(ref name, size)| (name.to_owned(), size))
        .collect();
    for plugin_id in new_master_ids {
        if remap_table.contains_key(
            &(plugin_id
                .checked_add(masters_len)
                .with_context(|| format!("Bug: overflow adding plugin_id = \"{plugin_id}\" to masters_len = \"{masters_len}\""))?),
        ) {
            let new_master = &plugins_processed
                .get(
                    usize::try_from(*plugin_id)
                        .with_context(|| format!("Bug: failed to cast \"{plugin_id}\"(plugin_id, u32) to usize"))?,
                )
                .with_context(|| format!("Bug: indexing slicing plugins_processed[{plugin_id}]"))?;
            let size = match new_master.path.metadata() {
                Ok(meta) => meta.len(),
                Err(error) => {
                    let text = format!(
                        "Info: Failed to get the size of \"{}\" with error: \"{:#}\". Master's size was set to 0.",
                        new_master.path.display(),
                        error
                    );
                    msg(text, 0, cfg, log)?;
                    0
                }
            };
            new_masters.push((new_master.name.clone(), size));
        }
    }
    Ok(new_masters)
}

fn make_del_ref_master_renum_num_cells(
    del_ref_num_cells: NumCells,
    master_remap_table: &MasterRemapTable,
) -> Result<NumCellsRefCounted> {
    #[allow(clippy::cast_possible_truncation)]
    let mut cells_with_refs_per_cell: Vec<(usize, u32, Cell)> = del_ref_num_cells
        .into_par_iter()
        .map(|(id, cell)| -> Result<(usize, u32, Cell)> {
            let mut renum_references: HashMap<(MastId, RefrId), Reference> = HashMap::new();
            for (pair, reference) in cell.references {
                match master_remap_table.get(&reference.mast_index) {
                    None => renum_references.insert(pair, reference),
                    Some(master_id_mapping) => renum_references.insert(
                        (*master_id_mapping, pair.1),
                        Reference {
                            mast_index: *master_id_mapping,
                            ..reference
                        },
                    ),
                };
            }
            Ok((
                id,
                u32::try_from(renum_references.len()).with_context(|| {
                    format!(
                        "Bug: failed to cast {renum_references:?}(renum_references, usize) to u32"
                    )
                })?,
                Cell {
                    references: renum_references,
                    ..cell
                },
            ))
        })
        .collect::<Result<_>>()?;
    cells_with_refs_per_cell.sort_by_key(|x| x.0);
    let mut del_ref_master_renum_num_cells: NumCellsRefCounted = Vec::new();
    let mut ref_sum = 0;
    for (id, ref_num, cell) in cells_with_refs_per_cell {
        del_ref_master_renum_num_cells.push((id, ref_sum, cell));
        ref_sum = ref_sum.checked_add(ref_num).with_context(|| {
            format!("Bug: overflow adding ref_num = \"{ref_num}\" to ref_sum = \"{ref_sum}\"")
        })?;
    }
    Ok(del_ref_master_renum_num_cells)
}

fn make_grass_cells(del_ref_master_renum_num_cells: &NumCellsRefCounted) -> Result<GrassCells> {
    let mut num_grass_cells = del_ref_master_renum_num_cells
        .par_iter()
        .map(|&(id, ref_sum, ref cell)| -> Result<(usize, Cell)> {
            let mut refr_index = ref_sum;
            let mut grass_references: HashMap<(MastId, RefrId), Reference> = HashMap::new();
            let mut local_references: Vec<&Reference> = cell.references.values().collect();
            references_sorted(&mut local_references);
            for reference in local_references {
                refr_index = increment!(refr_index);
                grass_references.insert(
                    (0, refr_index),
                    Reference {
                        mast_index: 0,
                        refr_index,
                        deleted: None,
                        ..reference.clone()
                    },
                );
            }
            Ok((
                id,
                Cell {
                    references: grass_references,
                    ..cell.clone()
                },
            ))
        })
        .collect::<Result<NumCells>>()?;
    num_grass_cells.sort_by_key(|x| x.0);
    Ok(num_grass_cells
        .into_iter()
        .map(|(_, cell)| cell)
        .collect::<GrassCells>())
}

fn make_del_ref_cells(del_ref_master_renum_num_cells: NumCellsRefCounted) -> DelRefCells {
    let mut del_ref_cells = del_ref_master_renum_num_cells
        .into_iter()
        .map(|(_, _, cell)| cell)
        .collect::<DelRefCells>();
    for cell in &mut del_ref_cells {
        for reference in cell.references.values_mut() {
            reference.translation = [0.0; 3];
            reference.rotation = [0.0; 3];
            reference.scale = None;
        }
    }
    del_ref_cells
}

fn make_tng_statics(name: &str, h: &Helper, cfg: &Cfg, log: &mut Log) -> Result<Vec<Static>> {
    if h.g.turn_normal_grass.is_empty() {
        return Ok(Vec::new());
    }
    let mut tng_statics: Vec<Static> = Vec::new();
    let mut tngs: Vec<&TurnNormalGrass> = h.g.turn_normal_grass.values().collect();
    let tngs_stat_count: usize = tngs.iter().map(|x| x.stat_records.len()).sum();
    let first = tngs
        .first()
        .with_context(|| "Bug: indexing slicing tngs[0]")?;
    let first_path = first.new_path.to_string_lossy();
    let meshes_dir = first_path.strip_suffix(&first.new_name_low);
    let mut text =
        format!("  {tngs_stat_count} grass STAT records were added to plugin {name:?}(check log or add -vv to get detailed list)");
    if cfg.verbose < 2 {
        msg_no_log(&text, 1, cfg);
    }
    if let Some(path) = meshes_dir {
        write!(
            text,
            ":\n    (Meshes paths are relative to directory \"{path}\")"
        )?;
    };
    tngs.sort_by_key(|x| &x.new_name_low);
    for tng in tngs {
        for stat in &tng.stat_records {
            let mesh = tng.new_name_low.replace('/', "\\");
            write!(
                text,
                "\n    STAT \"{}\" points to added mesh \"{}\" copied from {}",
                stat.id, mesh, tng.src_info
            )?;
            tng_statics.push(Static {
                mesh,
                ..stat.clone()
            });
        }
    }
    msg(text, 2, cfg, log)?;
    Ok(tng_statics)
}

#[allow(clippy::too_many_arguments, clippy::shadow_reuse)]
fn make_tng_plugin(
    name: &str,
    cells: Vec<Cell>,
    statics: Option<Vec<Static>>,
    masters: Vec<(String, u64)>,
    header_text: HeaderText,
    h: &Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<Plugin> {
    let mut plugin = Plugin::new();
    let mut num_records = u32::try_from(cells.len()).with_context(|| {
        format!(
            "Bug: failed to cast {:?}(cells.len(), usize) to u32",
            cells.len()
        )
    })?;
    if let Some(ref statics) = statics {
        num_records = num_records
            .checked_add(
                u32::try_from(statics.len())
                    .with_context(|| format!("Bug: failed to cast {:?}(statics.len(), usize) to u32", statics.len()))?,
            )
            .with_context(|| {
                format!(
                    "Bug: overflow adding statics.len() = \"{}\" to num_records = \"{num_records}\"",
                    statics.len()
                )
            })?;
    };
    // COMMENT: statics.is_none() for -CONTENT which always contains external refs
    let strip_masters = if statics.is_none() {
        if h.g.list_options.strip_masters {
            let text = format!("Output plugin \"{name}\": masters will not be stripped due to encountering external reference");
            msg(text, 0, cfg, log)?;
        }
        false
    } else {
        h.g.list_options.strip_masters
    };
    let header = make_header(
        name,
        masters,
        num_records,
        strip_masters,
        header_text,
        cfg,
        log,
    )
    .with_context(|| format!("Failed to make header for plugin \"{name}\""))?;
    plugin.objects.push(TES3Object::Header(header));
    if let Some(statics) = statics {
        for stat in statics {
            plugin.objects.push(TES3Object::Static(stat));
        }
    }
    for cell in cells {
        plugin.objects.push(TES3Object::Cell(cell));
    }
    Ok(plugin)
}
