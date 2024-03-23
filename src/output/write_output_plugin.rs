use crate::{msg, msg_no_log, references_sorted, Cfg, Helper, Log, Mode, StatsUpdateKind};
use anyhow::{anyhow, Context, Result};
use fs_err::create_dir_all;
use std::{
    fmt::Write as _,
    path::{Path, PathBuf},
};
use tes3::esp::{Cell, CellFlags, FixedString, Header, Plugin, Reference, TES3Object};

pub fn write_output_plugin(
    name: &str,
    plugin: &mut Plugin,
    old_plugin: &mut Plugin,
    level: u8,
    h: &mut Helper,
    cfg: &Cfg,
    log: &mut Log,
) -> Result<()> {
    let mut plugins_differ_insignificantly = false;
    let (no_compare, dry_run) = get_no_compare_and_dry_run(level, h)?;
    let name_path = PathBuf::from(name);
    if name_path.exists() {
        if !no_compare {
            old_plugin
                .load_path(name)
                .with_context(|| format!("Failed to read previous output plugin {name:?}"))?;
            let (is_plugin_equal, mut text) =
                is_plugin_equal_to_previous(name, plugin, old_plugin, &mut plugins_differ_insignificantly)?;
            if is_plugin_equal {
                if cfg.verbose < 1 {
                    msg_no_log(&text, 0, cfg);
                }
                text = format!("{}{}", text, cfg.guts.prefix_list_stats);
                msg(text, 1, cfg, log)?;
                add_missing_plugin_stats(level, &plugin.objects, h)?;
                return Ok(());
            }
            msg(text, 0, cfg, log)?;
            old_plugin.objects.clear();
        }
    } else if let Some(out_dir) = name_path.parent() {
        if out_dir != Path::new("") && !out_dir.exists() {
            if !dry_run {
                create_dir_all(out_dir)
                    .with_context(|| format!("Failed to create output plugin directory \"{}\"", out_dir.display()))?;
            }
            let text = if dry_run {
                format!("Output plugin directory \"{}\" would be created", out_dir.display())
            } else {
                format!("Output plugin directory \"{}\" was created", out_dir.display())
            };
            msg(text, 0, cfg, log)?;
        }
    } else { //
    };
    if !dry_run {
        plugin
            .save_path(name)
            .with_context(|| format!("Failed to write output plugin to {name:?}"))?;
    }
    let mut text = if !dry_run {
        format!("Output plugin {name:?} was written")
    } else if h.g.list_options.dry_run_dismiss_stats && level == 1 {
        h.g.stats_dismiss = true;
        String::new()
    } else {
        format!("Output plugin {name:?} would be written")
    };
    if !text.is_empty() && h.g.contains_non_external_refs && level == 1 && !matches!(h.g.list_options.mode, Mode::Grass) {
        if plugins_differ_insignificantly {
            if dry_run {
                write!(text, ". New game would not be required.")?;
            } else {
                write!(text, ". New game is not required.")?;
            }
        } else if !dry_run {
            write!(text, ". It contains reindexed references most likely, so new game is recommended.")?;
        } else {
            write!(
                text,
                ". It would contain reindexed references most likely, so new game would be recommended."
            )?;
        }
    };
    add_missing_plugin_stats(level, &plugin.objects, h)?;
    if !text.is_empty() {
        if cfg.verbose < 1 {
            msg_no_log(&text, 0, cfg);
        }
        text = format!("{}{}", text, cfg.guts.prefix_list_stats);
        msg(text, 1, cfg, log)?;
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn is_plugin_equal_to_previous(
    name: &str,
    new_plugin: &Plugin,
    old_plugin: &Plugin,
    almost_equal: &mut bool,
) -> Result<(bool, String)> {
    let mut almost_equal_text = String::new();
    let mut only_size_of_masters_changed = false;
    let mut text = format!("Output plugin {name:?} differs from previous version. First difference is: ");
    if new_plugin.objects.len() != old_plugin.objects.len() {
        write!(
            text,
            "records number was changed from \"{}\" to \"{}\".",
            old_plugin.objects.len(),
            new_plugin.objects.len()
        )?;
        return Ok((false, text));
    }
    for (new, old) in new_plugin.objects.iter().zip(old_plugin.objects.iter()) {
        #[allow(clippy::wildcard_enum_match_arm)]
        match *new {
            TES3Object::Header(ref new_header) => {
                if let TES3Object::Header(ref old_header) = *old {
                    if new_header != old_header {
                        let new_header_stripped = strip_author_description_master_sizes(new_header);
                        let old_header_stripped = strip_author_description_master_sizes(old_header);
                        if new_header_stripped == old_header_stripped {
                            almost_equal_text = format!("Output plugin {name:?} differs from previous version insignificantly:");
                            if new_header.author != old_header.author {
                                *almost_equal = true;
                                write!(
                                    almost_equal_text,
                                    "\n  Author field was changed from \"{}\" to \"{}\"",
                                    *old_header.author, *new_header.author,
                                )?;
                            };
                            if new_header.description != old_header.description {
                                *almost_equal = true;
                                write!(
                                    almost_equal_text,
                                    "\n  Description field was changed from \"{}\" to \"{}\"",
                                    *old_header.description, *new_header.description,
                                )?;
                            };
                            if new_header.masters != old_header.masters {
                                if !*almost_equal {
                                    almost_equal_text = format!(
                                        "Output plugin {name:?} is equal to previous version, only size of master(s) was changed:"
                                    );
                                }
                                for (&(ref master, new_size), &(_, old_size)) in
                                    new_header.masters.iter().zip(old_header.masters.iter())
                                {
                                    if new_size != old_size {
                                        only_size_of_masters_changed = true;
                                        write!(
                                            almost_equal_text,
                                            "\n  Size of master \"{master}\" was changed from \"{old_size}\" to \"{new_size}\""
                                        )?;
                                    }
                                }
                            };
                        } else {
                            if new_header_stripped.num_objects != old_header_stripped.num_objects {
                                write!(
                                    text,
                                    "records number was changed from \"{}\" to \"{}\" in header.",
                                    old_header_stripped.num_objects, new_header_stripped.num_objects,
                                )?;
                            } else if new_header_stripped.masters.len() != old_header_stripped.masters.len() {
                                write!(
                                    text,
                                    "masters number was changed from \"{}\" to \"{}\" in header.",
                                    old_header_stripped.masters.len(),
                                    new_header_stripped.masters.len(),
                                )?;
                            } else if new_header_stripped.masters != old_header_stripped.masters {
                                write!(text, "masters list was changed in header.")?;
                            } else {
                                write!(text, "header was changed.")?;
                            }
                            return Ok((false, text));
                        }
                    }
                } else {
                    write!(
                        text,
                        "header was moved(strange, looks like either old version was broken or new version is broken)."
                    )?;
                    return Ok((false, text));
                }
            }
            TES3Object::Cell(ref new_cell) => {
                if let TES3Object::Cell(ref old_cell) = *old {
                    if new_cell.flags != old_cell.flags
                        || new_cell.name != old_cell.name
                        || new_cell.data != old_cell.data
                        || new_cell.region != old_cell.region
                        || new_cell.map_color != old_cell.map_color
                        || new_cell.water_height != old_cell.water_height
                        || new_cell.atmosphere_data != old_cell.atmosphere_data
                    {
                        write!(text, "cell properties were changed in \"{}\".", get_cell_name(new_cell))?;
                        return Ok((false, text));
                    };
                    let mut new_refs: Vec<&Reference> = new_cell.references.values().collect();
                    references_sorted(&mut new_refs);
                    let mut old_refs: Vec<&Reference> = old_cell.references.values().collect();
                    references_sorted(&mut old_refs);
                    if new_refs != old_refs {
                        write!(text, "references were changed in \"{}\".", get_cell_name(new_cell))?;
                        return Ok((false, text));
                    }
                } else {
                    write!(text, "previous version didn't contain cell \"{}\".", get_cell_name(new_cell))?;
                    return Ok((false, text));
                }
            }
            _ => {
                if new != old {
                    write!(text, "at least one non-cell record was changed.")?;
                    return Ok((false, text));
                }
            }
        }
    }
    Ok(if *almost_equal {
        (false, almost_equal_text)
    } else if only_size_of_masters_changed {
        (true, almost_equal_text)
    } else {
        (true, format!("Output plugin {name:?} is equal to previous version"))
    })
}

fn get_no_compare_and_dry_run(level: u8, h: &Helper) -> Result<(bool, bool)> {
    match level {
        1 => Ok((h.g.list_options.no_compare, h.g.list_options.dry_run)),
        2 => Ok((h.g.list_options.no_compare_secondary, h.g.list_options.dry_run_secondary)),
        _ => Err(anyhow!("Bug: wrong plugin operation level passed to write_output_plugin function")),
    }
}

fn add_missing_plugin_stats(level: u8, plugin_objects: &[TES3Object], h: &mut Helper) -> Result<()> {
    #[allow(clippy::wildcard_enum_match_arm)]
    if level == 1 {
        for object in plugin_objects {
            match *object {
                TES3Object::Cell(ref cell) => {
                    h.g.stats.instances_total_add_count(cell.references.len())?;
                }
                _ => continue,
            }
        }
    } else {
        for object in plugin_objects {
            match *object {
                TES3Object::Header(_) => h.g.stats_tng.tes3(StatsUpdateKind::ResultUnique),
                TES3Object::Static(_) => h.g.stats_tng.stat(StatsUpdateKind::ResultUnique),
                TES3Object::Cell(ref cell) => {
                    h.g.stats_tng.cell(StatsUpdateKind::ResultUnique);
                    h.g.stats_tng.instances_total_add_count(cell.references.len())?;
                }
                _ => continue,
            }
        }
        h.g.stats_tng.add_result_plugin()?;
    }
    Ok(())
}

fn get_cell_name(cell: &Cell) -> String {
    if cell.data.flags.contains(CellFlags::IS_INTERIOR) {
        cell.name.clone()
    } else {
        format!("{:?}", cell.data.grid)
    }
}

fn strip_author_description_master_sizes(header: &Header) -> Header {
    #[allow(clippy::pattern_type_mismatch)]
    Header {
        author: FixedString(String::new()),
        description: FixedString(String::new()),
        masters: header
            .masters
            .iter()
            .map(|(name, _)| (name.to_owned(), 0_u64))
            .collect::<Vec<(String, u64)>>(),
        ..header.clone()
    }
}
