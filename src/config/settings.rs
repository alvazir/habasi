use super::SettingsFile;
use crate::read_lines;
use anyhow::{Context, Result};
use confique::Config;
mod advanced;
mod guts;
mod options;
use advanced::Advanced;
use guts::Guts;
use options::Options;

#[derive(Config)]
pub(super) struct Settings {
    #[config(nested)]
    pub(super) options: Options,
    #[config(nested)]
    pub(super) advanced: Advanced,
    #[config(nested)]
    pub(super) guts: Guts,
}

fn check_settings_version(settings_file: &mut SettingsFile) -> Result<()> {
    if settings_file.path.exists() {
        let settings_toml_lines = read_lines(&settings_file.path).with_context(|| {
            format!(
                "Failed to read program configuration file \"{}\"",
                &settings_file.path.display()
            )
        })?;
        let settings_version_prefix = "# # Settings version: ";
        let expected_settings_version = String::from("0.3.4");
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

pub(in crate::config) fn get_settings(settings_file: &mut SettingsFile) -> Result<Settings> {
    let settings = Settings::builder()
        .file(&settings_file.path)
        .load()
        .with_context(|| {
            "Failed to load settings. Try to recreate settings file or run without it."
        })?;
    check_settings_version(settings_file)?;
    Ok(settings)
}
