use anyhow::{Context, Result};
use confique::Config;
use std::path::PathBuf;

#[derive(Config)]
pub(crate) struct Settings {
    #[config(nested)]
    pub(crate) options: Options,
    #[config(nested)]
    pub(crate) guts: Guts,
}

#[derive(Config)]
pub(crate) struct Options {
    /// Description of all the options is provided with --help. There are two lines per each option: default value and set value. Uncomment second line for the needed option and set the value.
    ///
    /// Example of multiple merged plugins:
    ///
    /// merge = [
    /// [
    /// "MergedGhostRevenge.esp",
    /// "GhostRevenge.ESP",
    /// "GhostRevenge_TR1912.esp",
    /// ],
    /// [
    /// "MergedPlugin01.esp",
    /// "replace",
    /// "Frozen in Time.esp",
    /// "The Minotaurs Ring.esp",
    /// "Cure all the Kwama Queens.ESP",
    /// ],
    /// ]
    ///
    /// Example of merging everything:
    ///
    /// merge = [
    /// [
    /// "/home/alvazir/__OMW/sbox/Data Files/United-ALL.esp",
    /// "complete_replace",
    /// "dry_run",
    /// "base_dir:/home/alvazir/__OMW/game/Morrowind/Data Files",
    /// "strip_masters",
    /// "Morrowind.esm",
    /// "Tribunal.esm",
    /// "Bloodmoon.esm",
    /// "/home/alvazir/__OMW/mods/Leveling/Delevel/delev.esp",
    /// ],
    /// ]
    ///
    /// Windows-style paths with backslash symbol '\' require special care. You may:
    ///   - Replace backslash with slash, e.g. '\' => '/'
    ///   - Prepend backslash with another slash(so-called escaping), e.g. '\' => '\\'
    ///   - Enclose string into single quotes instead of double quotes. If path contains single quote itself, then enclose string into triple single quotes
    ///   Examples:
    ///     - "D:/Data Files" = "D:\\Data Files" = 'D:\Data Files' = '''D:\Data Files'''
    ///     - "C:/mods/mod with quote'.esp" = "C:\\mods\\mod with quote'.esp" = '''C:\mods\mod with quote'.esp'''
    ///
    /// Example of different styles of Windows paths:
    ///
    /// merge = [
    /// [
    /// "United-100.esp",
    /// "C:/Data Files/Morrowind.esm",
    /// "C:\\Data Files\\Tribunal.esm",
    /// 'C:\Data Files\Bloodmoon.esm',
    /// '''C:\Data Files\mod with quote' in name.esp''',
    /// ],
    /// ]
    ///
    /// Available per list options:
    ///
    /// - mode:
    ///     
    ///     "keep"(default), "keep_without_lands", "replace", "complete_replace", "grass"
    ///
    /// - base_dir:
    ///
    ///     "base_dir:off"(default), "base_dir:<PATH>"
    ///
    /// - dry run:
    ///
    ///     "dry_run", "no_dry_run".
    ///
    /// - no ignore errors:
    ///
    ///     "no_ignore_errors", "ignore_errors".
    ///     
    /// - strip masters:
    ///     
    ///     "strip_masters", "no_strip_masters".
    ///
    /// - reindex:
    ///
    ///     "reindex", "no_reindex".
    ///
    /// - debug:
    ///     
    ///     "debug", "no_debug".
    ///
    #[config(default = [])]
    pub(crate) merge: Vec<Vec<String>>,
    #[config(default = false)]
    pub(crate) no_compare: bool,
    #[config(default = true)]
    pub(crate) grass: bool,
    #[config(default = "")]
    pub(crate) log: String,
    #[config(default = false)]
    pub(crate) no_log: bool,
    ///
    /// Global list options. Per list options provided in "merge" section override these values.
    ///
    #[config(default = false)]
    pub(crate) dry_run: bool,
    #[config(default = "keep")]
    pub(crate) mode: String,
    #[config(default = "base_dir:off")]
    pub(crate) base_dir: String,
    #[config(default = false)]
    pub(crate) no_ignore_errors: bool,
    #[config(default = false)]
    pub(crate) strip_masters: bool,
    #[config(default = false)]
    pub(crate) reindex: bool,
    #[config(default = false)]
    pub(crate) debug: bool,
    ///
    /// [Verbosity]
    /// Verbosity number corresponds to the number of verbose flags passed, e.g. -v = 1, -vv = 2, -vvv = 3.
    ///
    #[config(default = 0)]
    pub(crate) verbose: u8,
    #[config(default = false)]
    pub(crate) quiet: bool,
    #[config(default = false)]
    pub(crate) show_all_missing_refs: bool,
}

#[derive(Config)]
pub(crate) struct Guts {
    /// Guts of the program. Use at your own risk ;-)
    ///
    /// [Grass]
    /// This filter works only in "grass" mode. By default it filters out "UNKNOWN_GRASS" records
    /// from Remiros Groundcover. It's possible to filter more by adding to the list(i.e. if you
    /// don't like some kind of grass or added mushrooms etc). Values are case insensitive.
    ///
    #[config(default = ["unknown_grass"])]
    pub(crate) grass_filter: Vec<String>,
    ///
    /// [Header]
    /// Output plugin will have these values placed into header.
    ///
    #[config(default = 1.3)]
    pub(crate) header_version: f32,
    #[config(default = "Habasi")]
    pub(crate) header_author: String,
    #[config(default = "Auto-merged plugin")]
    pub(crate) header_description: String,
    ///
    /// [Messages]
    /// Unsorted parts of messages used in multiple places.
    ///
    #[config(default = "Combined plugin lists stats:")]
    pub(crate) prefix_combined_stats: String,
    #[config(default = ". Stats:")]
    pub(crate) prefix_list_stats: String,
}

pub(crate) fn get_settings(settings_toml: &PathBuf) -> Result<Settings> {
    let settings = Settings::builder()
        .file(settings_toml)
        .load()
        .with_context(|| "Failed to load settings")?;
    Ok(settings)
}
