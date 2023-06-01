use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(
    author,
    version,
    verbatim_doc_comment,
    after_long_help = "Notes:
  - Display/log output looks better with monospaced font.
  - Don't clean the output plugin. It's not designed to be cleaned.
  - Cell references added by merged plugins(unlike external references coming from masters) are reindexed, so starting new game is required to use such merged plugins. Similar message is displayed for every written output plugin that contains internal non-external references."
)]
/// Habasi - TES3 Plugin Merging Tool
///
/// Author: alvazir
/// License: Unlicense OR MIT
/// GitHub: https://github.com/alvazir/habasi
/// Nexus Mods: https://www.nexusmods.com/morrowind/mods/53002
pub(crate) struct Options {
    /// List(s) of plugins to merge. This option is handy for one-shot merges. Settings file should be more convenient for "permanent" or longer lists , see --settings.
    ///
    /// Each list is a double-quoted(*) string that consists of output plugin name, optional list options("replace" in second example) and comma-separated list of plugins to merge. Ouput plugin's name should come first. Examples:
    ///   "MergedGhostRevenge.esp, GhostRevenge.ESP, GhostRevenge_TR1912.esp"
    ///   "MergedPlugin01.esp, replace, Frozen in Time.esp, The Minotaurs Ring.esp, Cure all the Kwama Queens.ESP"
    ///
    /// May be repeated. May take either one or multiple comma-separated lists(no space after comma). Following examples are identical:
    ///   -m "MergedGhostRevenge.esp, GhostRevenge.ESP, GhostRevenge_TR1912.esp" -m "MergedPlugin01.esp, replace, Frozen in Time.esp, The Minotaurs Ring.esp, Cure all the Kwama Queens.ESP"
    ///   --merge "MergedGhostRevenge.esp, GhostRevenge.ESP, GhostRevenge_TR1912.esp","MergedPlugin01.esp, replace, Frozen in Time.esp, The Minotaurs Ring.esp, Cure all the Kwama Queens.ESP"
    ///
    /// List options may be set globally and per each list. List specific options override global options. See each of the list options details in corresponding help sections.
    ///
    /// (*) Windows-style paths with backslash symbol '\' require special care. You may:
    ///   - Replace backslash with slash, e.g. '\' => '/'
    ///   - Prepend backslash with another slash(so-called escaping), e.g. '\' => '\\'
    ///   - Enclose string into single quotes instead of double quotes. If path contains single quote itself, then enclose string into triple single quotes
    ///   Examples:
    ///     - "D:/Data Files" = "D:\\Data Files" = 'D:\Data Files' = '''D:\Data Files'''
    ///     - "C:/mods/mod with quote'.esp" = "C:\\mods\\mod with quote'.esp" = '''C:\mods\mod with quote'.esp'''
    #[arg(
        conflicts_with = "settings_write",
        short,
        long,
        help = "List(s) of plugins to merge",
        value_name = "OUTPUT[, OPTIONS], LIST",
        num_args = 1..,
        verbatim_doc_comment,
    )]
    pub(crate) merge: Option<Vec<String>>,
    /// Do not compare output plugin with previous version if it exists.
    ///
    /// By default program doesn't overwrite previous version of output plugin if it's not changed. Disabling comparison would slightly improve processing time.
    #[arg(
        conflicts_with = "settings_write",
        short = 'C',
        long,
        help = "Do not compare output plugin with previous version"
    )]
    pub(crate) no_compare: bool,
    /// Process grass lists(enabled by default).
    ///
    /// Grass rarely changes and it's processing may take more time then other plugins combined due to the size. Consider setting this option to "false" in settings file and then use this flag sometimes.
    #[arg(conflicts_with = "settings_write", short, long, help = "Process grass lists(enabled by default)")]
    pub(crate) grass: bool,
    /// Name of the log file. May be provided as a path. Non-existent directories will be created.
    ///
    /// Log contains display output of the program as if it was run with maximum verboseness. It is enabled by default, use --no-log to disable.
    ///
    /// Default value: "<program_name>.log"(file will be created in program directory).
    #[arg(
        short,
        long,
        value_name = "PATH",
        value_hint = clap::ValueHint::Other,
        help = "Name of the log file"
    )]
    pub(crate) log: Option<String>,
    /// Do not write log.
    #[arg(short = 'L', long, help = "Do not write log")]
    pub(crate) no_log: bool,
    /// Name of the program settings file. May be provided as a path. Non-existent directories will be created. Extension will be replaced with ".toml".
    ///
    /// Default value: "<program_name>.toml"(file will be created in program directory).
    #[arg(
        short,
        long,
        value_name = "PATH",
        value_hint = clap::ValueHint::FilePath,
        help = "Name of the program settings file"
    )]
    pub(crate) settings: Option<String>,
    /// Write default program settings file and exit.
    ///
    /// Use this option if you keep using the same arguments. Modify default settings to suit your needs.
    ///
    /// File will be created in program directory with name "<program_name>.toml" by default. Use --settings to provide another path. Keep in mind that non-default settings file path should be explicitly provided every time you want to use it.
    ///
    /// This flag conflicts with everything except --settings, --log, --no-log.
    #[arg(long, help = "Write default program settings file and exit")]
    pub(crate) settings_write: bool,
    /// Do not write output plugin(s).
    ///
    /// Corresponding per list options: "dry_run", "no_dry_run".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short,
        long,
        help = "Do not write output plugin(s)"
    )]
    pub(crate) dry_run: bool,
    /// Mode defines how to process possibly mergeable record. Available modes are:
    ///
    ///   "keep"
    ///     - All possibly mergeable records are stacked in the output plugin, so that record merging utilities(TES3Merge, Delta Plugin, Merged Lands etc) would be able to do their work as if plugins were not merged together. Nothing would break if you don't use any record merging utilities at all.
    ///
    ///   "keep_without_lands"
    ///     - Same as "keep", but LAND records(landscape) would simply be replaced. You may use this mode if you don't intend to use "Merged Lands".
    ///
    ///   "replace"
    ///     - All records are replaced(hence no records to merge, that's how Morrowind works with records), except leveled lists. Leveled list merging utilities(tes3cmd, OMWLLF, Jobasha, TES3Merge, Delta Plugin etc) would be able to do their work as if plugins were not merged together. You may use this mode if you don't merge any records except leveled lists. Engine processes merged plugins of "keep" and "replace" modes exactly the same, but "replace" produces slightly smaller results.
    ///
    ///   "complete_replace"
    ///     - Same as replace, but leveled lists are replaced too. You may use this mode if you don't merge anything. I'd only recommend this mode for minimalistic mode setups, but why'd you need plugin merger then?
    ///
    ///   "grass"
    ///     - Same as replace, but designed for grass. Allows excluding instances that you don't like. By default it excludes "UNKNOWN GRASS" records from Remiros' Groundcover. Check "grass_filter" option in settings file if you want to exclude anything else(i.e. mushrooms).
    ///
    /// Default value: "keep".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short = 'M',
        long,
        help = "How to process possibly mergeable records",
        verbatim_doc_comment
    )]
    pub(crate) mode: Option<String>,
    /// Base directory for plugin lists.
    ///
    /// By default program uses current directory("base_dir:off") as a base for plugin's relative paths. Plugin's absolute paths obviously ignore this option.
    ///
    /// Example:
    ///   -m "BTBGIsation - Custom Merged.esp, base_dir:mods/Patches/BTBGIsation/03 Modular - Secondary, BTBGIsation - Magical Missions.esp, BTBGIsation - Weapons Expansion Morrowind.esp"
    ///
    /// Default value: "base_dir:off".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short = 'B',
        long,
        help = "Base directory for plugin lists",
        value_name = "base_dir:PATH",
        verbatim_doc_comment
    )]
    pub(crate) base_dir: Option<String>,
    /// Do not ignore non-important errors.
    ///
    /// By default program ignores external references that are missing in master, mimicing game engines behaviour. Those references are simply not placed into the output plugin.
    ///
    /// Corresponding per list options: "no_ignore_errors", "ignore_errors".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short = 'I',
        long,
        help = "Do not ignore non-important errors"
    )]
    pub(crate) no_ignore_errors: bool,
    /// Strip masters when possible.
    ///
    /// Master-file subrecords are placed into the output plugin's header. They are not strictly required for some plugins, e.g. grass plugins or any other plugin that doesn't have external cell references. Program would strip master subrecords when enabled. It's all or nothing operation. One or more of external cell references would result in keeping all the master subrecords.
    ///
    /// Corresponding per list options: "strip_masters", "no_strip_masters".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short = 'S',
        long,
        help = "Strip masters when possible"
    )]
    pub(crate) strip_masters: bool,
    /// Reindex references twice.
    ///
    /// References are numbered as they appear by default. Cells would contain non-continious ranges of reference ids as a result. Use this option to reindex references again at the expense of additional processing time(up to 30%). This option doesn't change anything gameplay wise, it only makes output plugin internally look like it was produced by TES-CS.
    ///
    /// Corresponding per list options: "reindex", "no_reindex".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short = 'R',
        long,
        help = "Reindex references twice"
    )]
    pub(crate) reindex: bool,
    /// All versions of record would be placed into the output plugin. May be useful for investigating record mutations.
    ///
    /// Corresponding per list options: "debug", "no_debug".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short = 'D',
        long,
        help = "Debug"
    )]
    pub(crate) debug: bool,
    /// Show more information. May be provided multiple times for extra effect:
    ///
    ///   -v: Show list options, total stats per list, "references reindexed" and "master subrecords stripped" messages.
    ///
    ///   -vv: Show detailed total stats, ignored reference errors, "processing plugin" messages.
    ///
    ///   -vvv: Show detailed list stats.
    ///
    /// This flag conflicts with --quiet.
    #[arg(
        help_heading = "Display output",
        conflicts_with_all = ["settings_write", "quiet"],
        short,
        long,
        action = clap::ArgAction::Count,
        help = "Show more information",
        verbatim_doc_comment,
    )]
    pub(crate) verbose: u8,
    /// Do not show anything.
    ///
    /// This flag conflicts with --verbose.
    #[arg(
        help_heading = "Display output",
        conflicts_with_all = ["settings_write", "verbose"],
        short,
        long,
        help = "Do not show anything"
    )]
    pub(crate) quiet: bool,
    /// Show all missing references.
    ///
    /// By default only first missing reference per cell is logged to prevent noise.
    #[arg(
        help_heading = "Display output",
        conflicts_with = "settings_write",
        short = 'a',
        long,
        help = "Show all missing references"
    )]
    pub(crate) show_all_missing_refs: bool,
}

pub(crate) fn get_options() -> Result<Options> {
    let options = Options::try_parse()?;
    Ok(options)
}
