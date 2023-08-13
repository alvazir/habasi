use crate::{err_or_ignore_thread_safe, msg_no_log, Assets, Bsa, Cfg, FileInBsa, LoadOrder};
use anyhow::{Context, Result};
use hashbrown::{hash_map::Entry, HashMap};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

pub(crate) fn get_loose_meshes(load_order: &LoadOrder, assets: &mut Assets, ignore_important_errors: bool, cfg: &Cfg) -> Result<()> {
    let mut found_files: Vec<(usize, String, PathBuf)> = load_order
        .datas
        .par_iter()
        .map(|(id, dir_path)| -> Result<Vec<(usize, String, PathBuf)>, _> {
            let mut res: Vec<(usize, String, PathBuf)> = Vec::new();
            let mut broken_symlinks = Vec::new();
            for entry in WalkDir::new(dir_path)
                .follow_links(true)
                .into_iter()
                .filter_entry(|e| !is_not_meshes_dir(e, &cfg.guts.meshes_dir.string))
            {
                match entry {
                    Ok(entry) => {
                        if !entry.file_type().is_dir() {
                            let path = entry.into_path();
                            if let Some(file_extension) = path.extension() {
                                if file_extension.eq_ignore_ascii_case(&cfg.guts.mesh_extension.os_string) {
                                    let mut relative_path = PathBuf::new();
                                    let mut path_components = Vec::new();
                                    for component in path.iter().rev() {
                                        let component_low = component.to_ascii_lowercase();
                                        if component_low != cfg.guts.meshes_dir.os_string {
                                            path_components.push(component_low)
                                        } else {
                                            for i in path_components.iter().rev() {
                                                relative_path.push(i);
                                            }
                                            res.push((*id, relative_path.to_string_lossy().into_owned(), path));
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(error) => {
                        if error.depth() == 0 {
                            let text = format!("Failed to open directory \"{}\" with error: \"{:#}\"", dir_path.display(), error);
                            err_or_ignore_thread_safe(text, ignore_important_errors, cfg)?;
                        } else {
                            match error.path() {
                                None => {
                                    let text = format!(
                                        "Something went wrong while reading contents of directory \"{}\" with error: \"{:#}\"",
                                        dir_path.display(),
                                        error
                                    );
                                    err_or_ignore_thread_safe(text, ignore_important_errors, cfg)?;
                                }
                                Some(path) => {
                                    if !path.is_symlink() {
                                        let text = format!(
                                            "Failed to read \"{}\" in directory \"{}\" with error: \"{:#}\"",
                                            path.display(),
                                            dir_path.display(),
                                            error
                                        );
                                        err_or_ignore_thread_safe(text, ignore_important_errors, cfg)?;
                                    } else {
                                        broken_symlinks.push(path.to_string_lossy().into_owned());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if !broken_symlinks.is_empty() {
                if broken_symlinks.len() == 1 {
                    msg_no_log(
                        format!("Warning: ignored {} broken symlink: {}", broken_symlinks.len(), broken_symlinks[0],),
                        0,
                        cfg,
                    );
                } else {
                    let mut text = format!(
                        "Warning: ignored {} broken symlinks(use --verbose to list them)",
                        broken_symlinks.len()
                    );
                    msg_no_log(&text, 0, cfg);
                    text = "  Broken symlink: ".to_string();
                    text.push_str(&broken_symlinks.join("\n  Broken symlink: "));
                    msg_no_log(text, 1, cfg);
                }
            }
            Ok(res)
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|vec| !vec.is_empty())
        .flatten()
        .collect();

    found_files.sort();
    let mut all_files: HashMap<String, PathBuf> = HashMap::new();
    found_files.into_iter().rev().for_each(|(_, file_name_lowercased, path)| {
        if let Entry::Vacant(v) = all_files.entry(file_name_lowercased) {
            v.insert(path);
        }
    });
    assets.meshes.loose.scanned = true;
    assets.meshes.loose.files = all_files;
    Ok(())
}

pub(crate) fn get_bsa_meshes(load_order: &LoadOrder, assets: &mut Assets, cfg: &Cfg) -> Result<()> {
    read_bsas(load_order, assets).with_context(|| "Failed to read BSA archives")?;
    let mut found_files: Vec<(usize, String, FileInBsa)> = load_order
        .fallback_archives
        .par_iter()
        .map(|(bsa_index, _, _)| -> Result<Vec<(usize, String, FileInBsa)>, _> {
            let mut res: Vec<(usize, String, FileInBsa)> = Vec::new();
            for (file_index, name) in assets.bsa[*bsa_index].names.iter().enumerate() {
                if name.ends_with(&cfg.guts.mesh_extension.string) {
                    let mut relative_path = PathBuf::new();
                    let mut path_components = Vec::new();
                    for component in Path::new(&name.replace('\\', "/")).iter().rev() {
                        if component != cfg.guts.meshes_dir.os_string {
                            path_components.push(component)
                        } else {
                            for i in path_components.iter().rev() {
                                relative_path.push(i);
                            }
                            break;
                        }
                    }
                    res.push((
                        *bsa_index,
                        relative_path.to_string_lossy().into_owned(),
                        FileInBsa {
                            path: name.to_owned(),
                            bsa_index: *bsa_index,
                            file_index,
                        },
                    ));
                }
            }
            Ok(res)
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|vec| !vec.is_empty())
        .flatten()
        .collect();
    found_files.sort_by_key(|x| x.0);
    let mut all_files: HashMap<String, FileInBsa> = HashMap::new();
    found_files.into_iter().rev().for_each(|(_, file_name_lowercased, file_in_bsa)| {
        if let Entry::Vacant(v) = all_files.entry(file_name_lowercased) {
            v.insert(file_in_bsa);
        }
    });
    assets.meshes.bsa.scanned = true;
    assets.meshes.bsa.files = all_files;
    Ok(())
}

fn is_not_meshes_dir(entry: &DirEntry, meshes_dir: &str) -> bool {
    entry.depth() == 1
        && entry.file_type().is_dir()
        && entry
            .file_name()
            .to_str()
            .map(|s| !s.eq_ignore_ascii_case(meshes_dir))
            .unwrap_or(false)
}

fn read_bsas(load_order: &LoadOrder, assets: &mut Assets) -> Result<()> {
    let mut res: Vec<(usize, Bsa)> = load_order
        .fallback_archives
        .par_iter()
        .map(|(index, path, _)| -> Result<(usize, Bsa), _> {
            let bsa = Bsa::new(path).with_context(|| format!("Failed to read BSA file \"{}\"", path))?;
            Ok((*index, bsa))
        })
        .collect::<Result<_>>()?;
    res.sort_by_key(|x| x.0);
    res.into_iter().for_each(|bsa| assets.bsa.push(bsa.1));
    Ok(())
}
