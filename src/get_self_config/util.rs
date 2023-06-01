use crate::Mode;
use anyhow::{anyhow, Result};
use std::{
    env::current_exe,
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

pub(crate) fn get_settings_file(exe: &Option<String>, dir: &Option<PathBuf>, name: &Option<String>) -> Result<PathBuf> {
    let extension = "toml";
    let fallback_filename = "settings.toml";
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
    let settings_file = match name {
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
