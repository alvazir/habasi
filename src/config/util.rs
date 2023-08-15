use super::{Options, SettingsFile, StringOsPath, TngStatIds};
use crate::{read_lines, Mode};
use anyhow::{anyhow, Context, Result};
use hashbrown::{hash_map::Entry, HashMap, HashSet};
use std::{
    env::current_exe,
    fs::copy,
    path::{Path, PathBuf},
};

pub(crate) fn get_exe_name_and_dir() -> (Option<String>, Option<PathBuf>) {
    match current_exe() {
        Ok(path) => (
            path.file_stem().map(|exe| exe.to_string_lossy().into_owned()),
            path.parent().map(|dir| dir.to_owned()),
        ),
        Err(_) => (None, None),
    }
}

pub(crate) fn get_settings_file(exe: &Option<String>, dir: &Option<PathBuf>, options: &Options) -> Result<SettingsFile> {
    let extension = "toml";
    let fallback_filename = "settings.toml";
    let name = &options.settings;
    let filename = match name {
        Some(name) => match Path::new(name).file_stem() {
            Some(filename) => format!("{}.{extension}", filename.to_string_lossy()),
            None => return Err(anyhow!("Failed to get settings filename without extension from \"{}\"", name)),
        },
        None => match exe {
            Some(file_stem) => format!("{file_stem}.{extension}"),
            None => {
                eprintln!("Failed to get program name: falling back to default name \"{fallback_filename}\" for settings");
                fallback_filename.into()
            }
        },
    };
    let settings_file_path = match name {
        Some(name) => match Path::new(name).parent() {
            Some(path) => path.join(filename),
            None => PathBuf::from(&filename),
        },
        None => match dir {
            Some(path) => path.join(filename),
            None => {
                eprintln!("Failed to get program directory: falling back to checking \"{filename}\" in current directory");
                PathBuf::from(filename)
            }
        },
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

pub(crate) fn get_log_file(no_log: bool, name: String, exe: Option<String>, dir: Option<PathBuf>) -> Result<Option<PathBuf>> {
    if no_log {
        return Ok(None);
    }
    let extension = "log";
    let fallback_filename = "log.log";
    let filename = match name.is_empty() {
        false => match Path::new(&name).file_name() {
            Some(filename) => filename.to_string_lossy().into_owned(),
            None => return Err(anyhow!("Failed to get log file name \"{}\"", name)),
        },
        true => match exe {
            Some(file_stem) => format!("{file_stem}.{extension}"),
            None => {
                eprintln!("Failed to get program name: falling back to default name \"{fallback_filename}\" for log");
                fallback_filename.into()
            }
        },
    };
    let log = match name.is_empty() {
        false => match Path::new(&name).parent() {
            Some(path) => path.join(filename),
            None => PathBuf::from(&filename),
        },
        true => match dir {
            Some(path) => path.join(filename),
            None => {
                eprintln!("Failed to get program directory: falling back to writing log into \"{filename}\" in current directory");
                PathBuf::from(filename)
            }
        },
    };
    Ok(Some(log))
}

pub(crate) fn get_lists(opt: Option<Vec<String>>, set: Vec<Vec<String>>) -> Vec<Vec<String>> {
    match opt {
        None => set,
        Some(list_strings) => {
            let mut lists = Vec::new();
            for string in list_strings {
                lists.push(string.split(',').map(|element| element.trim().to_owned()).collect::<Vec<String>>());
            }
            lists
        }
    }
}

pub(crate) fn check_mode(mode_str: &str) -> Result<Mode> {
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

pub(crate) fn check_base_dir(base_dir_string: &str) -> Result<PathBuf> {
    match base_dir_string.trim() {
        "" => Ok(PathBuf::new()),
        _ => {
            let base_dir = PathBuf::from(base_dir_string);
            if !base_dir.exists() {
                Err(anyhow!("Provided base_dir doesn't exist: \"{}\"", base_dir_string))
            } else {
                Ok(base_dir)
            }
        }
    }
}

pub(crate) fn set_low_string_osstring(string: String) -> StringOsPath {
    let low = string.to_lowercase();
    StringOsPath {
        string: low.clone(),
        path_buf: low.clone().into(),
        os_string: low.into(),
    }
}

pub(crate) fn make_tng_stat_ids(list: Vec<String>, separator: &str) -> Result<TngStatIds> {
    let mut set = HashSet::new();
    let mut source_map = HashMap::new();
    for line in list.into_iter() {
        let split_line: Vec<_> = line.split(separator).collect();
        if split_line.len() == 2 {
            let fallback_plugin = split_line[0].to_lowercase();
            let stat_id = split_line[1].to_lowercase();
            source_map.insert(stat_id.clone(), fallback_plugin.clone());
            set.insert(stat_id);
        } else {
            return Err(anyhow!("Error: settings.advanced.turn_normal_grass_stat_ids line is incorrect\n  Should be \"fallback_plugin{0}name_of_static\", e.g. \"Morrowind.esm{0}Flora_kelp_01\"\n  Incorrect line is \"{1}\"", separator, line));
        }
    }
    Ok(TngStatIds { set, source_map })
}

pub(crate) fn set_new_name_retries(num: u8) -> Result<u8> {
    if num > 1 {
        Ok(num)
    } else {
        Err(anyhow!(
            "Error: settings.guts.turn_normal_grass_new_name_retries should be larger than \"1\", though it's set to \"{}\" now",
            num
        ))
    }
}

pub(crate) fn check_settings_version(settings_file: &mut SettingsFile) -> Result<()> {
    if settings_file.path.exists() {
        let settings_toml_lines = read_lines(&settings_file.path)
            .with_context(|| format!("Failed to read program configuration file \"{}\"", &settings_file.path.display()))?;
        let settings_version_prefix = "# # Settings version: ";
        let expected_settings_version = String::from("0.2.0");
        let mut detected_settings_version = String::from("0.1.0");
        for line in settings_toml_lines.flatten() {
            if line.starts_with(settings_version_prefix) {
                let version_raw = &line.strip_prefix(settings_version_prefix);
                if let Some(version_raw) = version_raw {
                    detected_settings_version = version_raw.trim().to_owned();
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

pub(crate) fn backup_settings_file(settings_file: &mut SettingsFile, backup_suffix: &str) -> Result<u64> {
    if settings_file.path.exists() {
        let mut backup_path = settings_file.path.clone().into_os_string();
        backup_path.push(backup_suffix);
        settings_file.backup_path = backup_path.into();
        settings_file.backup_overwritten = settings_file.backup_path.exists();
        settings_file.backup_written = true;
        copy(&settings_file.path, &settings_file.backup_path).with_context(|| {
            format!(
                "Failed to backup program settings \"{}\" to \"{}\"",
                &settings_file.path.display(),
                &settings_file.backup_path.display()
            )
        })
    } else {
        Ok(0)
    }
}

pub(crate) fn make_keep_only_last_info_ids(list: Vec<Vec<String>>) -> Result<HashMap<String, HashMap<String, String>>> {
    let mut res = HashMap::new();
    for (n, line) in list.into_iter().enumerate() {
        let line_len = line.len();
        if line_len < 2 {
            let description = "Should contain at least 2 subelements [\"ID\", \"Topic\"]";
            return Err(anyhow!(make_keep_only_last_info_ids_err_text(description, n, &line)));
        } else if line_len > 3 {
            let description = "Should contain no more than 3 subelements [\"ID\", \"Topic\", \"Reason\"]";
            return Err(anyhow!(make_keep_only_last_info_ids_err_text(description, n, &line)));
        }
        let id = line[0].clone();
        if !id.chars().all(|b| "0123456789".contains(b)) {
            let description = "ID should only contain digits(0-9)";
            return Err(anyhow!(make_keep_only_last_info_ids_err_text(description, n, &line)));
        }
        let topic = line[1].to_lowercase();
        let reason = if line_len == 3 {
            line[2].clone()
        } else {
            String::from("Reason not defined.")
        };
        match res.entry(id) {
            Entry::Vacant(v) => {
                v.insert(HashMap::from([(topic, reason)]));
            }
            Entry::Occupied(mut o) => {
                let value = o.get_mut();
                if let Some(b) = value.insert(topic, reason) {
                    let description = &format!("There is already a pair of \"ID\" and \"Topic\" with \"Reason\": \"{}\"", b);
                    return Err(anyhow!(make_keep_only_last_info_ids_err_text(description, n, &line)));
                }
            }
        };
    }
    Ok(res)
}

fn make_keep_only_last_info_ids_err_text(description: &str, line_num: usize, line: &[String]) -> String {
    format!(
        "Error: settings.advanced.keep_only_last_info_ids element \"{}\" is incorrect\nDescription: {description}\nElement: [{}]",
        line_num + 1,
        line.iter().map(|x| format!("\"{x}\"")).collect::<Vec<_>>().join(", ")
    )
}
