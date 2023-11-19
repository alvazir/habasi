use super::{get_bsa_meshes, get_loose_meshes};
use crate::{msg, Cfg, FallbackStatics, Helper, Log, Out, TurnNormalGrass};
use anyhow::{anyhow, Context, Result};
use fs_err::{create_dir_all, read, File};
use hashbrown::{hash_map::Entry, hash_set::Entry as SetEntry, HashMap, HashSet};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use std::{
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};
use tes3::esp::{Plugin, Static};

pub(crate) fn make_tng_meshes(mut dir: PathBuf, out: &Out, h: &mut Helper, cfg: &Cfg, log: &mut Log) -> Result<()> {
    select_meshes(out, h, h.g.config_index, cfg, log).with_context(|| "Failed to select meshes to use as grass")?;
    read_meshes(h, h.g.config_index).with_context(|| "Failed to read meshes that would be used as grass")?;
    get_new_mesh_names(&mut dir, h, h.g.config_index, cfg, log)
        .with_context(|| "Failed to make names for newly added grass meshes")?;
    write_meshes(dir, h, cfg, log).with_context(|| "Failed to write new grass meshes")?;
    Ok(())
}

fn select_meshes(out: &Out, h: &mut Helper, idx: usize, cfg: &Cfg, log: &mut Log) -> Result<()> {
    for stat_id in &h.g.found_stat_ids {
        let stat = match h.g.r.stat.get(stat_id) {
            Some(index) => &out.stat[*index].0,
            None => {
                let fallback_plugin = cfg.advanced.turn_normal_grass_stat_ids.source_map.get(stat_id).unwrap();
                if h.g.plugins_processed.iter().any(|x| &x.name_low == fallback_plugin) {
                    return Err(anyhow!(
                        "Failed to find STAT record \"{stat_id}\", fallback plugin \"{fallback_plugin}\" was already processed"
                    ));
                }
                if !h.t.fallback_statics[idx].contains_key(fallback_plugin) {
                    let mut success = false;
                    for plugin_name in &h.t.game_configs[idx].load_order.contents {
                        if plugin_name.to_lowercase().ends_with(fallback_plugin) {
                            success =
                                get_fallback_statics(stat_id, plugin_name, fallback_plugin, &mut h.t.fallback_statics[idx], cfg, log)
                                    .with_context(|| "Failed to read fallback plugin \"{fallback_plugin}\"")?;
                            break;
                        }
                    }
                    if !success {
                        return Err(anyhow!("Failed to find STAT record \"{stat_id}\". Failed to find fallback plugin \"{fallback_plugin}\". Make sure that it's in load order."));
                    }
                };
                let stat = match h.t.fallback_statics[idx].get(fallback_plugin) {
                    None => return Err(anyhow!("Failed to find STAT record \"{stat_id}\". Failed to process fallback plugin \"{fallback_plugin}\". It should be a bug.")),
                    Some(v) => match v.0.get(stat_id) {
                        None => return Err(anyhow!("Failed to find STAT record \"{stat_id}\". Failed to find it in fallback plugin \"{fallback_plugin}\" too.")),
                        Some(index) => &v.1[*index],
                    },
                };
                stat
            }
        };
        let mut mesh_path = PathBuf::new();
        for component_low in stat.mesh.to_lowercase().split(['/', '\\']) {
            mesh_path.push(component_low);
        }
        let mesh_name = mesh_path.to_string_lossy().into_owned();
        match h.g.turn_normal_grass.entry(mesh_name.clone()) {
            Entry::Vacant(v) => {
                if !h.t.assets[idx].meshes.loose.scanned {
                    get_loose_meshes(
                        &h.t.game_configs[idx].load_order,
                        &mut h.t.assets[idx],
                        h.g.list_options.ignore_important_errors,
                        cfg,
                    )
                    .with_context(|| "Failed to find loose meshes")?;
                }
                let loose = h.t.assets[idx].meshes.loose.files.get(&mesh_name).map(|x| x.to_owned());
                let mut bsa = None;
                if loose.is_none() || !h.g.list_options.prefer_loose_over_bsa {
                    if !h.t.assets[idx].meshes.bsa.scanned {
                        get_bsa_meshes(&h.t.game_configs[idx].load_order, &mut h.t.assets[idx], cfg)
                            .with_context(|| "Failed to find bsa meshes")?;
                    }
                    bsa = h.t.assets[idx].meshes.bsa.files.get(&mesh_name).cloned();
                }
                if loose.is_none() && bsa.is_none() {
                    return Err(anyhow!("Failed to find mesh file used by STAT record \"{}\"", stat.id));
                }
                v.insert(TurnNormalGrass {
                    stat_records: vec![stat.clone()],
                    loose,
                    bsa,
                    new_name_low: String::new(),
                    new_path: PathBuf::new(),
                    file_contents: Vec::new(),
                    src_info: String::new(),
                });
            }
            Entry::Occupied(mut o) => {
                o.get_mut().stat_records.push(stat.clone());
            }
        };
    }
    Ok(())
}

fn read_meshes(h: &mut Helper, idx: usize) -> Result<()> {
    h.g.turn_normal_grass
        .par_iter_mut()
        .map(|(_, turn_normal_grass)| -> Result<(), _> {
            if turn_normal_grass.loose.is_none() {
                turn_normal_grass.read_from_bsa(&h.t.assets[idx].bsa)?;
            } else if turn_normal_grass.bsa.is_none() || turn_normal_grass.should_read_from_loose(&h.t.game_configs[idx].load_order)? {
                turn_normal_grass.read_from_loose()?;
            } else {
                turn_normal_grass.read_from_bsa(&h.t.assets[idx].bsa)?;
            }
            Ok(())
        })
        .collect::<Result<_>>()?;
    Ok(())
}

fn get_new_mesh_names(dir: &mut PathBuf, h: &mut Helper, idx: usize, cfg: &Cfg, log: &mut Log) -> Result<()> {
    let failed_name_guess_message_verbosity = 3;
    msg(
        "  Picking names for newly added grass meshes:",
        failed_name_guess_message_verbosity,
        cfg,
        log,
    )?;
    let mut grass_meshes: HashSet<String> = HashSet::new();
    dir.push(&cfg.guts.meshes_dir.string);
    let dir_canonicalized = match dir.canonicalize() {
        Err(_) => dir.clone(),
        Ok(dir_canonicalized) => dir_canonicalized,
    };
    for (original_name, tng) in h.g.turn_normal_grass.iter_mut() {
        let path = Path::new(&original_name);
        let mut name_path = cfg.guts.grass_subdir.path_buf.clone();
        name_path.push(path.file_name().unwrap());
        let mut name = name_path.to_string_lossy().into_owned();
        for n in 0..cfg.guts.turn_normal_grass_new_name_retries {
            if n > 0 {
                if n < cfg.guts.turn_normal_grass_new_name_retries {
                    name_path = cfg.guts.grass_subdir.path_buf.clone();
                    name_path.push(format!("{}_{:02}.nif", path.file_stem().unwrap().to_string_lossy(), n));
                    name = name_path.to_string_lossy().into_owned();
                } else {
                    return Err(anyhow!(
                        "Failed to pick unique name for mesh \"{}\", last try was \"{}\"\nConsider increasing \"guts.turn_normal_grass_new_name_retries={}\" to a higher value", original_name, name, cfg.guts.turn_normal_grass_new_name_retries));
                }
            }
            if let Some(found_mesh_path) = h.t.assets[idx].meshes.loose.files.get(&name) {
                if !found_mesh_path.starts_with(&dir_canonicalized) {
                    let text = format!("    Will try again. Name \"{}\" picked for mesh \"{}\" doesn't fit,\n      because there is already the same name at path \"{}\".", name, original_name, found_mesh_path.display());
                    msg(text, failed_name_guess_message_verbosity, cfg, log)?;
                    continue;
                }
            }
            if let Some(found_mesh_path) = h.t.assets[idx].meshes.bsa.files.get(&name) {
                let text = format!("    Will try again. Name \"{}\" picked for mesh \"{}\" doesn't fit,\n      because there is already the same name in BSA \"{}\".", name, original_name, h.t.game_configs[idx].load_order.fallback_archives[found_mesh_path.bsa_index].1);
                msg(text, failed_name_guess_message_verbosity, cfg, log)?;
                continue;
            }
            match grass_meshes.entry(name.clone()) {
                SetEntry::Vacant(v) => {
                    v.insert();
                    tng.new_path = dir.join(&name);
                    tng.new_name_low = name;
                    break;
                }
                SetEntry::Occupied(o) => {
                    let text = format!("    Will try again. Name \"{}\" picked for mesh \"{}\" doesn't fit,\n      because there is already the same name produced by another new grass mesh at path \"{}\".", name, original_name, o.get());
                    msg(text, failed_name_guess_message_verbosity, cfg, log)?;
                    continue;
                }
            }
        }
    }
    dir.push(&cfg.guts.grass_subdir.string);
    Ok(())
}

fn write_meshes(dir: PathBuf, h: &mut Helper, cfg: &Cfg, log: &mut Log) -> Result<()> {
    if !dir.exists() {
        create_dir_all(&dir).with_context(|| format!("Failed to create output meshes directory \"{}\"", dir.display()))?;
    }
    let results: Vec<(bool, &PathBuf)> =
        h.g.turn_normal_grass
            .par_iter_mut()
            .map(|(_, turn_normal_grass)| -> Result<(bool, &PathBuf), _> {
                let mut old_data = Vec::new();
                if turn_normal_grass.new_path.exists() {
                    old_data = read(&turn_normal_grass.new_path)?;
                }
                if old_data != turn_normal_grass.file_contents {
                    let f = match File::create(&turn_normal_grass.new_path) {
                        Err(err) => {
                            return Err(anyhow!(
                                "Failed to {} file \"{}\" with error: \"{:#}\"",
                                if old_data.is_empty() { "create" } else { "truncate" },
                                &turn_normal_grass.new_path.display(),
                                err
                            ))
                        }
                        Ok(file) => file,
                    };
                    let mut f = BufWriter::new(f);
                    if f.write_all(&turn_normal_grass.file_contents).is_err() {
                        return Err(anyhow!("Failed to write into file \"{}\"", &turn_normal_grass.new_path.display(),));
                    }
                    if f.flush().is_err() {
                        return Err(anyhow!(
                            "Failed to finalize writing into file \"{}\"",
                            &turn_normal_grass.new_path.display(),
                        ));
                    };
                    Ok((true, &turn_normal_grass.new_path))
                } else {
                    Ok((false, &turn_normal_grass.new_path))
                }
            })
            .collect::<Result<Vec<_>>>()?;
    let total_count = results.len();
    let written_count = results.iter().filter(|(written, _)| *written).count();
    let untouched_count = total_count - written_count;
    let text = format!(
        "  New grass meshes were prepared: {}",
        if written_count > 0 && untouched_count > 0 {
            format!(
                "{} total, {} written, {} untouched. Check log to get detailed list.",
                total_count, written_count, untouched_count
            )
        } else if written_count > 0 {
            format!("{} written(check log or add --verbose to get detailed list)", written_count)
        } else {
            format!("{} untouched(check log or add -vv to get detailed list)", untouched_count)
        }
    );
    msg(text, 0, cfg, log)?;
    for (written, path) in results {
        match written {
            true => msg(format!("    Mesh was written: {}", path.display()), 1, cfg, log)?,
            false => msg(format!("    Mesh was untoched(already the same): {}", path.display()), 2, cfg, log)?,
        }
    }
    Ok(())
}

fn get_fallback_statics(
    stat_id: &str,
    plugin_name: &str,
    fallback_plugin: &str,
    fallback_statics: &mut FallbackStatics,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<bool> {
    let text = format!("Reading plugin \"{plugin_name}\" to get missing STAT \"{stat_id}\"");
    msg(text, 0, cfg, log)?;
    let mut plugin = Plugin::new();
    plugin
        .load_path(plugin_name)
        .with_context(|| format!("Failed to read plugin \"{}\"", plugin_name))?;
    let mut statics_index = HashMap::new();
    let mut statics = Vec::new();
    for record in plugin.objects_of_type::<Static>() {
        if cfg.advanced.turn_normal_grass_stat_ids.set.contains(&record.id.to_lowercase()) {
            statics_index.insert(record.id.to_lowercase(), statics.len());
            statics.push(record.clone());
        }
    }
    fallback_statics.insert(fallback_plugin.to_string(), (statics_index, statics));
    Ok(true)
}
