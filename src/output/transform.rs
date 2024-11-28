use crate::{
    get_cell_name, increment, msg, references_sorted, show_removed_record_ids, CellExtGrid, Cfg,
    Helper, Log, Mode, OldRefSources, Out, RefSources, StatsUpdateKind,
};
use anyhow::{anyhow, Context as _, Result};
use hashbrown::HashMap;
use rayon::iter::{IntoParallelRefMutIterator as _, ParallelIterator as _};
use tes3::esp::{Cell, CellFlags, ObjectFlags, Reference, Static};

pub fn transform(
    name: &str,
    mut out: Out,
    h: &mut Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<Out> {
    resort_skil_mgef(&mut out);
    set_creature_scale_to_none_if_default(&mut out);
    remove_ambi_whgt_from_deleted_cells(&mut out);
    if matches!(h.g.list_options.mode, Mode::Grass) {
        out.stat = exclude_non_grass_statics(out.stat, name, h, cfg, log)?;
        out.cell = exclude_interior_and_empty_cells(out.cell, name, h, cfg, log)?;
    }
    exclude_infos(&mut out, name, cfg, log)?;
    exclude_deleted_refs_mast_id_0(&mut out);
    if h.g.list_options.reindex {
        reindex_references(name, &mut out, h, cfg, log)?;
    }
    Ok(out)
}

fn exclude_non_grass_statics(
    src: Vec<(Static, Vec<Static>)>,
    name: &str,
    h: &mut Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<Vec<(Static, Vec<Static>)>> {
    let mut stats: Vec<(Static, Vec<Static>)> = Vec::with_capacity(src.len());
    let mut removed_record_ids = Vec::new();
    for stat in src {
        if stat.0.mesh.to_lowercase().starts_with("grass") {
            stats.push(stat);
        } else {
            removed_record_ids.push(format!(
                // "    Record STAT: {} was excluded from the result because it's not a grass static(mesh path {:?} doesn't start with \"grass\")",
                "    Record STAT: {} was excluded from the result because it's not a grass static(mesh path \"{}\" doesn't start with \"grass\")",
                &stat.0.id,
                &stat.0.mesh
            ));
            h.g.stats.stat(StatsUpdateKind::Excluded);
        }
    }
    show_removed_record_ids(
        &removed_record_ids,
        "\"grass\" mode and non-grass STAT",
        name,
        2,
        cfg,
        log,
    )?;
    Ok(stats)
}

fn exclude_interior_and_empty_cells(
    src: Vec<(Cell, Vec<Cell>)>,
    name: &str,
    h: &mut Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<Vec<(Cell, Vec<Cell>)>> {
    let mut cells: Vec<(Cell, Vec<Cell>)> = Vec::with_capacity(src.len());
    let mut removed_record_ids_interior = Vec::new();
    let mut removed_record_ids_empty = Vec::new();
    for cell in src {
        if cell.0.is_interior() {
            removed_record_ids_interior.push(format!(
                "    Record CELL: {} was excluded from the result because it's an interior cell",
                get_cell_name(&cell.0)
            ));
            h.g.stats.cell(StatsUpdateKind::Excluded);
        } else if cell.0.references.is_empty() {
            removed_record_ids_empty.push(format!(
                "    Record CELL: {} was excluded from the result because it's an empty cell",
                get_cell_name(&cell.0)
            ));
            h.g.stats.cell(StatsUpdateKind::Excluded);
        } else {
            cells.push(cell);
        }
    }
    show_removed_record_ids(
        &removed_record_ids_empty,
        "\"grass\" mode and interior cell",
        name,
        2,
        cfg,
        log,
    )?;
    show_removed_record_ids(
        &removed_record_ids_interior,
        "\"grass\" mode and empty cell",
        name,
        2,
        cfg,
        log,
    )?;
    Ok(cells)
}

#[allow(clippy::as_conversions)]
fn resort_skil_mgef(out: &mut Out) {
    out.skil.sort_by_key(|x| x.0.skill_id as i32);
    out.mgef.sort_by_key(|x| x.0.effect_id as i32);
}

fn set_creature_scale_to_none_if_default(out: &mut Out) {
    for &mut (ref mut last, ref mut prevs) in &mut out.crea {
        if last.scale == Some(1.0) {
            last.scale = None;
        }
        for prev in prevs.iter_mut() {
            if prev.scale == Some(1.0) {
                prev.scale = None;
            }
        }
    }
}

#[allow(clippy::too_many_lines)]
fn reindex_references(
    name: &str,
    out: &mut Out,
    h: &mut Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let mut dummy_ext_ref_sources: HashMap<CellExtGrid, (RefSources, OldRefSources)> =
        HashMap::new();
    let mut new_ext_ref_sources: HashMap<CellExtGrid, (RefSources, OldRefSources)> = HashMap::new();
    let mut refr = 1_u32;
    for &mut (ref mut last, _) in &mut out.cell {
        let mut reindexed_ext_ref_sources: RefSources = HashMap::new();
        let mut reindexed_ext_old_ref_sources: OldRefSources = HashMap::new();
        let is_ext_ref =
            h.g.list_options.turn_normal_grass && !last.data.flags.contains(CellFlags::IS_INTERIOR);
        let ext_ref_sources = if is_ext_ref {
            match h.g.r.ext_ref_sources.get(&last.data.grid) {
                None => {
                    return Err(anyhow!(
                        "Bug: failed to find cell \"{:?}\" in ext_ref_sources while reindexing",
                        &last.data.grid
                    ))
                }
                Some(ref_sources) => ref_sources,
            }
        } else {
            dummy_ext_ref_sources.insert(last.data.grid, (HashMap::new(), HashMap::new()));
            match dummy_ext_ref_sources.get(&last.data.grid) {
                None => {
                    return Err(anyhow!(
                    "Bug: failed to find cell \"{:?}\" in dummy_ext_ref_sources while reindexing",
                    &last.data.grid
                ))
                }
                Some(ref_sources) => ref_sources,
            }
        };

        let mut new_refs: HashMap<(u32, u32), Reference> = HashMap::new();
        let mut references: Vec<&Reference> = last.references.values().collect();
        references_sorted(&mut references);
        for reference in references {
            if reference.mast_index == 0 {
                let new_ref = Reference {
                    refr_index: refr,
                    ..reference.clone()
                };
                new_refs.insert((0_u32, refr), new_ref);
                if is_ext_ref {
                    match ext_ref_sources.0.get(&(reference.mast_index, reference.refr_index)) {
                        Some(v) => {
                            reindexed_ext_ref_sources.insert((0, refr), *v);
                        }
                        None => match ext_ref_sources.1.get(&(reference.mast_index, reference.refr_index)) {
                            Some(y) => {
                                reindexed_ext_old_ref_sources.insert((0, refr), y.clone());
                            }
                            None => {
                                return Err(anyhow!(
                                    "Bug: failed to find reference \"({}, {})\" in ext_ref_sources while reindexing",
                                    &reference.mast_index,
                                    &reference.refr_index
                                ))
                            }
                        },
                    }
                }
                refr = increment!(refr);
            } else {
                new_refs.insert(
                    (reference.mast_index, reference.refr_index),
                    reference.clone(),
                );
                if is_ext_ref {
                    match ext_ref_sources.0.get(&(reference.mast_index, reference.refr_index)) {
                        Some(v) => {
                            reindexed_ext_ref_sources.insert((reference.mast_index, reference.refr_index), *v);
                        }
                        None => match ext_ref_sources.1.get(&(reference.mast_index, reference.refr_index)) {
                            Some(y) => {
                                reindexed_ext_old_ref_sources.insert((reference.mast_index, reference.refr_index), y.clone());
                            }
                            None => {
                                return Err(anyhow!(
                                    "Bug: failed to find reference \"({}, {})\" in old_ext_ref_sources while reindexing",
                                    &reference.mast_index,
                                    &reference.refr_index
                                ))
                            }
                        },
                    }
                }
            }
        }
        last.references = new_refs;
        if is_ext_ref {
            new_ext_ref_sources.insert(
                last.data.grid,
                (reindexed_ext_ref_sources, reindexed_ext_old_ref_sources),
            );
        }
    }
    h.g.r.ext_ref_sources = new_ext_ref_sources;
    let text = format!("Output plugin {name:?}: references reindexed");
    msg(text, 1, cfg, log)?;
    Ok(())
}

fn exclude_infos(out: &mut Out, name: &str, cfg: &Cfg, log: &mut Log) -> Result<()> {
    let mut removed_record_ids = Vec::new();
    for &mut (ref mut dial, _) in &mut out.dial {
        if !dial.excluded_infos.is_empty() {
            dial.excluded_infos.sort_unstable();
            for n in dial.excluded_infos.iter().rev() {
                let info = dial.info.remove(*n);
                match cfg.advanced.keep_only_last_info_ids.get(&info.id) {
                        None => {
                            return Err(anyhow!(
                                "Bug: failed to find INFO ID \"{}\" in settings.advanced.keep_only_last_info_ids",
                                &info.id
                            ))
                        }
                        Some(topics) => match topics.get(&dial.dialogue.id.to_lowercase()) {
                            None => {
                                return Err(anyhow!(
                                    "Bug: failed to find DIAL \"{}\" for INFO ID \"{}\" in settings.advanced.keep_only_last_info_ids",
                                    &dial.dialogue.id,
                                    &info.id
                                ))
                            }
                            Some(reason) => removed_record_ids.push(
                                format!(
                                    "    Record INFO: non-last instance of \"{}\" from DIAL \"{}\" was excluded from the result\n      Reason: {reason})",
                                    &info.id,
                                    &dial.dialogue.id
                                    )
                                ),
                        }
                    }
            }
        }
    }
    show_removed_record_ids(
        &removed_record_ids,
        "settings.advanced.keep_only_last_info_ids",
        name,
        2,
        cfg,
        log,
    )?;
    Ok(())
}

fn exclude_deleted_refs_mast_id_0(out: &mut Out) {
    out.cell.par_iter_mut().for_each(|&mut (ref mut cell, _)| {
        if !cell.references.is_empty() {
            let references: Vec<&Reference> = cell.references.values().collect();
            let mut deleted_ref_indexes = Vec::new();
            for reference in &references {
                if reference.mast_index == 0 && reference.deleted.is_some() {
                    deleted_ref_indexes.push((reference.mast_index, reference.refr_index));
                }
            }
            if !deleted_ref_indexes.is_empty() {
                for deleted_ref_index in &deleted_ref_indexes {
                    cell.references.remove(deleted_ref_index);
                }
            }
        }
    });
}

fn remove_ambi_whgt_from_deleted_cells(out: &mut Out) {
    for &mut (ref mut cell, _) in &mut out.cell {
        if cell.is_interior() && cell.flags.contains(ObjectFlags::DELETED) {
            cell.water_height = None;
            cell.atmosphere_data = None;
        }
    }
}
