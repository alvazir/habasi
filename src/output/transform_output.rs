use crate::{
    get_cell_name, msg, references_sorted, show_removed_record_ids, CellExtGrid, Cfg, Helper, Log, Mode, OldRefSources, Out,
    RefSources, StatsUpdateKind,
};
use anyhow::{anyhow, Result};
use hashbrown::HashMap;
use tes3::esp::{Cell, Reference, Static};

pub(crate) fn transform_output(name: &str, mut out: Out, h: &mut Helper, cfg: &Cfg, log: &mut Log) -> Result<Out> {
    resort_skil_mgef(&mut out);
    set_creature_scale_to_none_if_default(&mut out);
    if matches!(h.g.list_options.mode, Mode::Grass) {
        out.stat = exclude_non_grass_statics(out.stat, name, h, cfg, log)?;
        out.cell = exclude_interior_and_empty_cells(out.cell, name, h, cfg, log)?;
    }
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
    for stat in src.into_iter() {
        if !stat.0.mesh.to_lowercase().starts_with("grass") {
            removed_record_ids.push(format!(
                "    Record STAT: {} was excluded from the result because it's not a grass static(mesh path \"{}\" doesn't start with \"grass\")",
                &stat.0.id,
                &stat.0.mesh
            ));
            h.g.stats.stat(StatsUpdateKind::Excluded);
        } else {
            stats.push(stat);
        }
    }
    show_removed_record_ids(removed_record_ids, "\"grass\" mode and non-grass STAT", name, 2, cfg, log)?;
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
    for cell in src.into_iter() {
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
    show_removed_record_ids(removed_record_ids_empty, "\"grass\" mode and interior cell", name, 2, cfg, log)?;
    show_removed_record_ids(removed_record_ids_interior, "\"grass\" mode and empty cell", name, 2, cfg, log)?;
    Ok(cells)
}

fn resort_skil_mgef(out: &mut Out) {
    out.skil.sort_by_key(|x| x.0.skill_id as i32);
    out.mgef.sort_by_key(|x| x.0.effect_id as i32);
}

fn set_creature_scale_to_none_if_default(out: &mut Out) {
    for (last, prevs) in out.crea.iter_mut() {
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

fn reindex_references(name: &str, out: &mut Out, h: &mut Helper, cfg: &Cfg, log: &mut Log) -> Result<()> {
    let mut dummy_ext_ref_sources: HashMap<CellExtGrid, (RefSources, OldRefSources)> = HashMap::new();
    let mut new_ext_ref_sources: HashMap<CellExtGrid, (RefSources, OldRefSources)> = HashMap::new();
    let mut refr = 1u32;
    for (last, _) in out.cell.iter_mut() {
        let mut reindexed_ext_ref_sources: RefSources = HashMap::new();
        let mut reindexed_ext_old_ref_sources: OldRefSources = HashMap::new();
        let is_ext_ref = h.g.list_options.turn_normal_grass && !last.data.flags.contains(tes3::esp::CellFlags::IS_INTERIOR);
        let ext_ref_sources = match is_ext_ref {
            false => {
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
            }
            true => match h.g.r.ext_ref_sources.get(&last.data.grid) {
                None => {
                    return Err(anyhow!(
                        "Bug: failed to find cell \"{:?}\" in ext_ref_sources while reindexing",
                        &last.data.grid
                    ))
                }
                Some(ref_sources) => ref_sources,
            },
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
                new_refs.insert((0u32, refr), new_ref);
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
                refr += 1;
            } else {
                new_refs.insert((reference.mast_index, reference.refr_index), reference.clone());
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
            new_ext_ref_sources.insert(last.data.grid, (reindexed_ext_ref_sources, reindexed_ext_old_ref_sources));
        }
    }
    h.g.r.ext_ref_sources = new_ext_ref_sources;
    let text = format!("Output plugin \"{}\": references reindexed", name);
    msg(text, 1, cfg, log)?;
    Ok(())
}
