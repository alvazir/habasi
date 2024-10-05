use crate::{msg, Cfg, Helper, Log};
use anyhow::{anyhow, Context, Result};
use dirs::{document_dir, preference_dir};
use std::path::PathBuf;

pub(super) fn get(h: &mut Helper, cfg: &Cfg, log: &mut Log) -> Result<()> {
    let config_path = if h.g.list_options.config.is_empty() {
        find_config(h, cfg, log).with_context(|| "Failed to find game configuration file")?
    } else {
        check_config(&h.g.list_options.config)
            .with_context(|| "Failed to read game configuration file")?
    };
    let config_path_canonical = config_path
        .canonicalize()
        .with_context(|| "Failed to canonicalize game configuration file path")?;
    if let Some(config_index) =
        h.t.game_configs
            .iter()
            .position(|config| config.path_canonical == config_path_canonical)
    {
        h.g.config_index = config_index;
    } else {
        h.g.config_index = h.t.game_configs.len();
        h.add_game_config(config_path, config_path_canonical);
    }
    Ok(())
}

fn find_config(h: &Helper, cfg: &Cfg, log: &mut Log) -> Result<PathBuf> {
    let mut checked_paths: Vec<PathBuf> = Vec::new();

    macro_rules! check_config_path {
        ($config_path:expr) => {
            if $config_path.exists() {
                let text = format!(
                    "Found game configuration file \"{}\"",
                    $config_path.display()
                );
                let verbosity = if h.t.game_configs.iter().any(|x| x.path == $config_path) {
                    u8::MAX
                } else {
                    0
                };
                msg(text, verbosity, cfg, log)?;
                return Ok($config_path);
            }
            checked_paths.push($config_path);
        };
    }
    if let Some(dir) = preference_dir() {
        check_config_path!(dir.join(&cfg.guts.config_path_suffix_linux_macos));
    } else {
        checked_paths.push(PathBuf::from(format!(
            "Failed to get __preference_dir__ to check \"__preference_dir__/{}\"",
            &cfg.guts.config_path_suffix_linux_macos
        )));
    };
    if let Some(dir) = document_dir() {
        check_config_path!(dir.join(&cfg.guts.config_path_suffix_windows));
    } else {
        checked_paths.push(PathBuf::from(format!(
            "Failed to get __document_dir__ to check \"__document_dir__/{}\"",
            &cfg.guts.config_path_suffix_windows
        )));
    };
    for path in &cfg.guts.config_paths_list {
        check_config_path!(PathBuf::new().join(path));
    }
    Err(anyhow!(
        "Failed to find game configuration file. Consider using --config-file option. Checked following paths:\n{}",
        checked_paths
            .iter()
            .map(|path| format!("\t{}", path.display()))
            .collect::<Vec<String>>()
            .join("\n")
    ))
}

fn check_config(config: &str) -> Result<PathBuf> {
    let config_path = PathBuf::from(&config);
    if !config_path.as_os_str().is_empty() && config_path.exists() {
        Ok(config_path)
    } else {
        Err(anyhow!(
            "Failed to find game configuration file at path \"{config}\""
        ))
    }
}
