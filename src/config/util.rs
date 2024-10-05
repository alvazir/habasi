use super::{Options, SettingsFile, StringOsPath, TngStatIds};
use crate::{increment, read_lines, Mode};
use anyhow::{anyhow, Context, Result};
use fs_err::rename;
use hashbrown::{hash_map::Entry, HashMap, HashSet};
use std::{
    env::current_exe,
    path::{Path, PathBuf},
};

pub fn get_exe_name_and_dir() -> (Option<String>, Option<PathBuf>) {
    current_exe().map_or((None, None), |path| {
        (
            path.file_stem()
                .map(|exe| exe.to_string_lossy().into_owned()),
            path.parent().map(ToOwned::to_owned),
        )
    })
}

pub fn get_settings_file(
    exe: &Option<String>,
    dir: &Option<PathBuf>,
    options: &Options,
) -> Result<SettingsFile> {
    let extension = "toml";
    let fallback_filename = "settings.toml";
    let name = &options.settings;
    #[allow(clippy::shadow_reuse, clippy::print_stderr)]
    let filename = match *name {
        Some(ref name) => match Path::new(name).file_stem() {
            Some(filename) => format!("{}.{extension}", filename.to_string_lossy()),
            None => return Err(anyhow!("Failed to get settings filename without extension from \"{}\"", name)),
        },
        None => exe.as_ref().map_or_else(
            || {
                eprintln!("Failed to get program name: falling back to default name \"{fallback_filename}\" for settings");
                fallback_filename.into()
            },
            |file_stem| format!("{file_stem}.{extension}"),
        ),
    };
    #[allow(clippy::shadow_reuse, clippy::print_stderr)]
    let settings_file_path = match *name {
        Some(ref name) => match Path::new(name).parent() {
            Some(path) => path.join(filename),
            None => PathBuf::from(&filename),
        },
        None => {
            if let Some(ref path) = *dir {
                path.join(filename)
            } else {
                eprintln!("Failed to get program directory: falling back to checking \"{filename}\" in current directory");
                PathBuf::from(filename)
            }
        }
    };
    let settings_file = SettingsFile {
        path: settings_file_path,
        version_message: None,
        write: options.settings_write,
        backup_path: PathBuf::new(),
        backup_written: false,
        backup_overwritten: false,
    };
    Ok(settings_file)
}

pub fn get_log_file(
    no_log: bool,
    name: &str,
    exe: Option<String>,
    dir: Option<PathBuf>,
) -> Result<Option<PathBuf>> {
    if no_log {
        return Ok(None);
    }
    let extension = "log";
    let fallback_filename = "log.log";
    let filename = if name.is_empty() {
        #[allow(clippy::print_stderr)]
        exe.map_or_else(
            || {
                eprintln!("Failed to get program name: falling back to default name \"{fallback_filename}\" for log");
                fallback_filename.into()
            },
            |file_stem| format!("{file_stem}.{extension}"),
        )
    } else {
        match Path::new(&name).file_name() {
            Some(filename) => filename.to_string_lossy().into_owned(),
            None => return Err(anyhow!("Failed to get log file name \"{}\"", name)),
        }
    };
    let log = if name.is_empty() {
        #[allow(clippy::print_stderr)]
        if let Some(path) = dir {
            path.join(filename)
        } else {
            eprintln!("Failed to get program directory: falling back to writing log into \"{filename}\" in current directory");
            PathBuf::from(filename)
        }
    } else {
        match Path::new(&name).parent() {
            Some(path) => path.join(filename),
            None => PathBuf::from(&filename),
        }
    };
    Ok(Some(log))
}

pub fn get_lists(
    opt: Option<Vec<String>>,
    set: Vec<Vec<String>>,
    arguments_tail: Vec<String>,
) -> Result<Vec<Vec<String>>> {
    opt.map_or(Ok(set), |list_strings| {
        let mut lists: Vec<Vec<String>> = Vec::new();
        let escape = '\\';
        let separator = ',';
        let separator_str_length = separator.len_utf8();
        let mut escaped_list_element = String::new();
        let mut string_slice: &str;
        let mut slice_start_offset: usize;
        #[allow(clippy::string_slice)]
        for string in list_strings {
            let mut list = Vec::new();
            slice_start_offset = 0;
            for (char_byte_offset, character) in string.char_indices() {
                if character == separator {
                    string_slice = string
                        .get(slice_start_offset..char_byte_offset)
                        .with_context(|| format!(
                                "Bug: indexing slicing string[{slice_start_offset}..{char_byte_offset}] in string = \"{string}\""
                                ))?;
                    if let Some(stripped_slice) = string_slice.strip_suffix(escape) {
                        if escaped_list_element.is_empty() {
                            escaped_list_element.push_str(stripped_slice.trim_start());
                        } else {
                            escaped_list_element.push_str(stripped_slice);
                        }
                        escaped_list_element.push(',');
                    } else {
                        push_list_element(string_slice, &mut escaped_list_element, &mut list);
                    }
                    slice_start_offset = char_byte_offset.checked_add(separator_str_length).with_context(|| {
                        format!(
                            "Bug: overflow adding separator_str_length = \"{separator_str_length}\" to char_byte_offset = \"{char_byte_offset}\" in string = \"{string}\"
                            ")
                    })?;
                }
            }
            string_slice = string
                .get(slice_start_offset..)
                .with_context(|| format!("Bug: indexing slicing string[{slice_start_offset}..] in string = \"{string}\""))?;
            push_list_element(string_slice, &mut escaped_list_element, &mut list);
            lists.push(list);
        }
        if !arguments_tail.is_empty() {
            if let Some(list) = lists.last_mut() {
                list.extend(arguments_tail);
            }
        }
        Ok(lists)
    })
}

fn push_list_element(
    string_slice: &str,
    escaped_list_element: &mut String,
    list: &mut Vec<String>,
) {
    if escaped_list_element.is_empty() {
        list.push(string_slice.trim().to_owned());
    } else {
        escaped_list_element.push_str(string_slice.trim_end());
        list.push(escaped_list_element.clone());
        escaped_list_element.clear();
    }
}

pub fn check_mode(mode_str: &str) -> Result<Mode> {
    let mode = match mode_str {
        "keep" => Mode::Keep,
        "keep_without_lands" => Mode::KeepWithoutLands,
        "jobasha" => Mode::Jobasha,
        "jobasha_without_lands" => Mode::JobashaWithoutLands,
        "grass" => Mode::Grass,
        "replace" => Mode::Replace,
        "complete_replace" => Mode::CompleteReplace,
        _ => return Err(anyhow!("Failed to parse provided mode \"{}\"", mode_str)),
    };
    Ok(mode)
}

pub fn check_base_dir(base_dir_string: &str) -> Result<PathBuf> {
    if base_dir_string.trim() == "" {
        Ok(PathBuf::new())
    } else {
        let base_dir = PathBuf::from(base_dir_string);
        if base_dir.exists() {
            Ok(base_dir)
        } else {
            Err(anyhow!(
                "Provided base_dir doesn't exist: \"{}\"",
                base_dir_string
            ))
        }
    }
}

pub fn set_low_string_osstring(string: &str) -> StringOsPath {
    let low = string.to_lowercase();
    StringOsPath {
        string: low.clone(),
        path_buf: low.clone().into(),
        os_string: low.into(),
    }
}

enum TngStatIdLinePart {
    FallbackPlugin,
    NameOfStatic,
    ExtraValue,
}

pub fn make_tng_stat_ids(list: Vec<String>, separator: &str) -> Result<TngStatIds> {
    let mut set = HashSet::new();
    let mut source_map = HashMap::new();
    for line in list {
        let mut split_line = line.split(separator);
        let fallback_plugin = parse_tng_stat_id_line(
            split_line.next(),
            &TngStatIdLinePart::FallbackPlugin,
            &line,
            separator,
        )?;
        let stat_id = parse_tng_stat_id_line(
            split_line.next(),
            &TngStatIdLinePart::NameOfStatic,
            &line,
            separator,
        )?;
        if split_line.next().is_none() {
            source_map.insert(stat_id.clone(), fallback_plugin);
            set.insert(stat_id);
        } else {
            return Err(tng_stat_id_line_error(
                "has more than 2 separated values",
                &TngStatIdLinePart::ExtraValue,
                &line,
                separator,
            ));
        }
    }
    Ok(TngStatIds { set, source_map })
}

fn parse_tng_stat_id_line(
    opt_string: Option<&str>,
    kind: &TngStatIdLinePart,
    line: &str,
    separator: &str,
) -> Result<String> {
    opt_string.map_or_else(
        || Err(tng_stat_id_line_error("is missing", kind, line, separator)),
        |value| {
            if value.is_empty() {
                Err(tng_stat_id_line_error("is empty", kind, line, separator))
            } else {
                Ok(value.to_lowercase())
            }
        },
    )
}

fn tng_stat_id_line_error(
    reason: &str,
    kind: &TngStatIdLinePart,
    line: &str,
    separator: &str,
) -> anyhow::Error {
    let (fallback_plugin, name_of_static, extra_value) =
        ("fallback plugin", "name of the static", "line");
    let kind_str = match *kind {
        TngStatIdLinePart::FallbackPlugin => fallback_plugin,
        TngStatIdLinePart::NameOfStatic => name_of_static,
        TngStatIdLinePart::ExtraValue => extra_value,
    };
    anyhow!("Error: settings.advanced.turn_normal_grass_stat_ids line is incorrect: {kind_str} {reason}\n  Should be \"{fallback_plugin}{separator}{name_of_static}\", e.g. \"Morrowind.esm{separator}Flora_kelp_01\"\n  Incorrect line is \"{line}\"")
}

pub fn set_new_name_retries(num: u8) -> Result<u8> {
    if num > 1 {
        Ok(num)
    } else {
        Err(anyhow!(
            "Error: settings.guts.turn_normal_grass_new_name_retries should be larger than \"1\", though it's set to \"{}\" now",
            num
        ))
    }
}

pub fn check_settings_version(settings_file: &mut SettingsFile) -> Result<()> {
    if settings_file.path.exists() {
        let settings_toml_lines = read_lines(&settings_file.path).with_context(|| {
            format!(
                "Failed to read program configuration file \"{}\"",
                &settings_file.path.display()
            )
        })?;
        let settings_version_prefix = "# # Settings version: ";
        let expected_settings_version = String::from("0.3.0");
        let mut detected_settings_version = String::from("0.1.0");
        for line in settings_toml_lines.map_while(Result::ok) {
            if line.starts_with(settings_version_prefix) {
                let version_raw = &line.strip_prefix(settings_version_prefix);
                #[allow(clippy::shadow_reuse)]
                if let Some(version_raw) = *version_raw {
                    version_raw
                        .trim()
                        .clone_into(&mut detected_settings_version);
                    break;
                }
            }
        }
        if detected_settings_version != expected_settings_version {
            settings_file.version_message =  Some(
                format!("Attention: Program configuration file \"{}\" version differs from expected:\n  Expected version = \"{}\", detected version = \"{}\".\n  Consider recreating it with \"--settings-write\".\n  File will be backed up and then overwritten, though better make backup yourself if you need it.", &settings_file.path.display(), expected_settings_version, detected_settings_version),
            );
        }
    }
    Ok(())
}

pub fn backup_settings_file(settings_file: &mut SettingsFile, backup_suffix: &str) -> Result<()> {
    if settings_file.path.exists() {
        let mut backup_path = settings_file.path.clone().into_os_string();
        backup_path.push(backup_suffix);
        settings_file.backup_path = backup_path.into();
        settings_file.backup_overwritten = settings_file.backup_path.exists();
        settings_file.backup_written = true;
        rename(&settings_file.path, &settings_file.backup_path).with_context(|| {
            format!(
                "Failed to rename previous program settings \"{}\" to \"{}\"",
                &settings_file.path.display(),
                &settings_file.backup_path.display()
            )
        })
    } else {
        Ok(())
    }
}

pub fn make_keep_only_last_info_ids(
    list: Vec<Vec<String>>,
) -> Result<HashMap<String, HashMap<String, String>>> {
    let mut res = HashMap::new();
    for (n, line) in list.into_iter().enumerate() {
        let line_len = line.len();
        #[allow(clippy::redundant_else)]
        if line_len < 2 {
            let description = "Should contain at least 2 subelements [\"ID\", \"Topic\"]";
            return Err(anyhow!(make_keep_only_last_info_ids_err_text(
                description,
                n,
                &line
            )?));
        } else if line_len > 3 {
            let description =
                "Should contain no more than 3 subelements [\"ID\", \"Topic\", \"Reason\"]";
            return Err(anyhow!(make_keep_only_last_info_ids_err_text(
                description,
                n,
                &line
            )?));
        } else {
            let id = line
                .first()
                .context("Bug: unreachable due to 2 <= line_len <= 3")?
                .clone();
            if !id.chars().all(|b| "0123456789".contains(b)) {
                let description = "ID should only contain digits(0-9)";
                return Err(anyhow!(make_keep_only_last_info_ids_err_text(
                    description,
                    n,
                    &line
                )?));
            }
            let topic = line
                .get(1)
                .context("Bug: unreachable due to 2 <= line_len <= 3")?
                .to_lowercase();
            let reason = if line_len == 3 {
                line.get(2)
                    .context("Bug: unreachable due to 2 <= line_len <= 3")?
                    .clone()
            } else {
                String::from("Reason not defined.")
            };
            match res.entry(id) {
                Entry::Vacant(v) => {
                    v.insert(HashMap::from([(topic, reason)]));
                }
                Entry::Occupied(mut o) => {
                    let value = o.get_mut();
                    #[allow(clippy::shadow_reuse)]
                    if let Some(reason) = value.insert(topic, reason) {
                        let description = &format!("There is already a pair of \"ID\" and \"Topic\" with \"Reason\": {reason:?}");
                        return Err(anyhow!(make_keep_only_last_info_ids_err_text(
                            description,
                            n,
                            &line
                        )?));
                    }
                }
            };
        }
    }
    Ok(res)
}

fn make_keep_only_last_info_ids_err_text(
    description: &str,
    line_num: usize,
    line: &[String],
) -> Result<String> {
    Ok(format!(
        "Error: settings.advanced.keep_only_last_info_ids element \"{}\" is incorrect\nDescription: {description}\nElement: [{}]",
        increment!(line_num),
        line.iter().map(|x| format!("\"{x}\"")).collect::<Vec<_>>().join(", ")
    ))
}

pub(in crate::config) fn prepare_plugin_extensions_to_ignore(list: &[String]) -> Vec<String> {
    let mut res = Vec::new();
    for extension in list {
        let mut prepared = extension.to_lowercase();
        prepared.insert(0, '.');
        res.push(prepared);
    }
    res
}
