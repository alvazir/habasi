use anyhow::{anyhow, Context, Result};
use clap::{builder::StyledStr, Arg, CommandFactory, Parser};

#[allow(clippy::doc_markdown, clippy::struct_excessive_bools)]
#[derive(Parser)]
#[command(
    author,
    version,
    verbatim_doc_comment,
    after_long_help = "Notes:
  - Display/log output looks better with monospaced font.
  - Don't clean the output plugin. It's not designed to be cleaned.
  - Cell references added by merged plugins(unlike external references coming from masters) are reindexed, so starting new game is required to use such merged plugins. Similar message is displayed for every written output plugin that contains non-external references."
)]
/// Habasi - TES3 plugin merging and utility tool
///
/// Author: alvazir
/// License: GNU GPLv3
/// GitHub: https://github.com/alvazir/habasi
/// Nexus Mods: https://www.nexusmods.com/morrowind/mods/53002
pub(in crate::config) struct Options {
    /// List(s) of plugins to merge. This option is handy for one-shot merges. Settings file should be more convenient for "permanent" or longer lists , see --settings. There are 2 variants of --merge argument, primary(1) and secondary(2).
    ///
    /// (1) Each list is a double-quoted(4) string that consists of output plugin name, optional list options("replace" in second example) and comma-separated list of plugins or plugin name patterns(3) to merge. Ouput plugin's name should come first. Examples:
    ///   "MergedGhostRevenge.esp, GhostRevenge.ESP, GhostRevenge_TR1912.esp"
    ///   "MergedPlugin01.esp, replace, Frozen in Time.esp, The Minotaurs Ring.esp, Cure all the Kwama Queens.ESP"
    ///
    ///   May be repeated. May take either one or multiple comma-separated lists(no space after comma). Following examples are identical:
    ///     -m "MergedGhostRevenge.esp, GhostRevenge.ESP, GhostRevenge_TR1912.esp" -m "MergedPlugin01.esp, replace, Frozen in Time.esp, The Minotaurs Ring.esp, Cure all the Kwama Queens.ESP"
    ///     --merge "MergedGhostRevenge.esp, GhostRevenge.ESP, GhostRevenge_TR1912.esp","MergedPlugin01.esp, replace, Frozen in Time.esp, The Minotaurs Ring.esp, Cure all the Kwama Queens.ESP"
    ///
    ///   List options may be set globally and per each list. List specific options override global options. See each of the list options details in corresponding help sections.
    ///
    /// (2) There is an alternative variant of --merge option. It allows to use shell's file name completion and wildcards(3). It may only be used once per command and should be placed last. Almost identical to main form, but doesn't require quoting the whole list and placing commas between items:
    ///     -m MergedGhostRevenge.esp GhostRevenge.ESP GhostRevenge_TR1912.esp
    ///
    /// (3) There are 3 types of plugin name patterns available:
    ///     
    ///   Wildcard: Usually provided by shell used(*nix), otherwise by the program(Windows). It's handy despite many limitations. Case-sensitivity depends on shell, results are unsorted, no file extension filtering. This option doesn't work correctly with --base-dir option, it may only be used with alternative variant of --merge. Example:
    ///     -m MergedGhostRevenge.esp GhostR*
    ///
    ///   Glob: Improved version of wildcard, independent of shell. Defined by prepending pattern with "glob:". Allows using "**" to get plugins from multiple subdirectories("glob:mods/**/*.esp"). Case-insensitive by default, sorted by modification time by default. Examples(produce the same result):
    ///     -m "MergedGhostRevenge.esp, glob:ghostr*"
    ///
    ///   Regex: The most powerful pattern type, though lacks glob's multi-directory access. Defined by prepending pattern with "regex:". Case-insensitive by default, sorted by modification time by default. Examples(produce the same result):
    ///     -m "MergedGhostRevenge.esp, regex:^ghostr.*"
    ///
    ///   All of these patterns may be mixed with plugin names and list options. Example:
    ///     -m "out.esp, replace, glob:**/*.omwaddon, regex:^repopulated.*es[mp], Animated_Morrowind - merged.esp" OAAB*Ship*TR* PB_AStrangePlant.esp
    ///
    ///   See --show-plugins, --regex_case_sensitive, --regex_sort_by_name if you use patterns a lot.
    ///
    /// (4) Windows-style paths with backslash symbol '\' require special care. You may:
    ///   - Replace backslash with slash, e.g. '\' => '/'
    ///   - Prepend backslash with another backslash(so-called escaping), e.g. '\' => '\\'
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
        verbatim_doc_comment
    )]
    pub(super) merge: Option<Vec<String>>,
    /// A dummy command that accepts any arguments. List of arguments is then appended to the last --merge list.
    #[arg(trailing_var_arg = true, hide = true)]
    pub(super) arguments_tail: Vec<String>,
    /// Name of the log file. May be provided as a path. Non-existent directories will be created.
    ///
    /// Log contains display output of the program as if it was run with maximum verboseness. It is enabled by default, use --no-log to disable. Previous log will be saved with ".previous" extension.
    ///
    /// Default value: "<program_name>.log"(file will be created in program directory).
    #[arg(
        short,
        long,
        value_name = "PATH",
        value_hint = clap::ValueHint::Other,
        help = "Name of the log file"
    )]
    pub(super) log: Option<String>,
    /// Do not write log.
    #[arg(short = 'L', long, alias = "no_log", help = "Do not write log")]
    pub(super) no_log: bool,
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
    pub(super) settings: Option<String>,
    /// Write default program settings file and exit.
    ///
    /// Use this option if you keep using the same arguments. Modify default settings to suit your needs.
    ///
    /// File will be created in program directory with name "<program_name>.toml" by default. Backup of old settings file will be saved with ".backup" extension. Use --settings to provide another path. Keep in mind that non-default settings file path should be explicitly provided every time you want to use it.
    ///
    /// This flag conflicts with everything except --settings, --log, --no-log, --verbose, --quiet.
    #[arg(long, aliases = ["settings_write", "write-settings", "write_settings"], help = "Write default program settings file and exit")]
    pub(super) settings_write: bool,
    /// Process grass lists(enabled by default).
    ///
    /// Grass rarely changes and it's processing may take more time then other plugins combined due to the size. Consider setting this option to "false" in settings file and then use this flag sometimes.
    #[arg(
        conflicts_with = "settings_write",
        short,
        long,
        help = "Process grass lists(enabled by default)"
    )]
    pub(super) grass: bool,
    /// Print help for specific option. Accepts both short and long option names.
    ///
    /// Long help(--help) is very long. Combining short help(-h) and this option(-?) is a convenient alternative.
    #[arg(
        short = '?',
        alias = "help_option",
        long,
        help = "Print help for specific option",
        value_name = "OPTION",
        allow_hyphen_values = true
    )]
    pub(super) help_option: Option<String>,
    /// Check for missing references in the whole load order.
    ///
    /// This preset is used to:
    ///   1. Find game config file
    ///   2. Get load order from it
    ///   3. Merge plugins and report missing references.
    ///   4. Produce no output plugin.
    ///
    /// Provide path to a game config file if program fails to find one. See --config.
    ///
    /// It's an alias to the following --merge string:
    ///   -m "CheckReferences.esp, dry_run, use_load_order, show_missing_refs, complete_replace, no_compare, ignore_errors, insufficient_merge, dry_run_dismiss_stats"
    ///
    /// As a preset:
    ///   1. It will ignore other --merge lists defined via command line arguments or settings file.
    ///   2. It may be combined with other presets.
    ///   3. Presets are a convenient way to quickly perform complex tasks. You may modify preset's output dir and plugin name(or anything else) in settings file. Though it's better to create similar "--merge" list in settings file with your own rules.
    #[arg(
        help_heading = "Presets",
        conflicts_with = "settings_write",
        short = 'C',
        long,
        alias = "preset_check_references",
        visible_alias = "check",
        help = "Check for missing references in the whole load order",
        verbatim_doc_comment
    )]
    pub(super) preset_check_references: bool,
    /// Turn Normal Grass and Kelp into Groundcover for the whole load order. See --turn-normal-grass for details.
    ///
    /// This preset is used to:
    ///   1. Find game config file
    ///   2. Get load order from it
    ///   3. Scan only cell references and statics
    ///   4. Produce 2 output plugins:
    ///     - "TurnNormalGrass-CONTENT.esp" with deleted references
    ///     - "TurnNormalGrass-GROUNDCOVER.esp" with new grass
    ///     - meshes folder with grass meshes(same meshes that were used for "normal" grass in your setup)
    ///
    /// Provide path to a game config file if program fails to find one. See --config.
    ///
    /// It's a convenience alias to the following --merge strings:
    ///   -m "TurnNormalGrass.esp, dry_run, use_load_order, turn_normal_grass, complete_replace, no_compare, ignore_errors, insufficient_merge, dry_run_dismiss_stats, no_show_missing_refs"
    ///      (when combined with preset-check-references):
    ///         - "show_missing_refs" is added on top
    ///
    /// As a preset:
    ///   1. It will ignore other "--merge" lists defined via command line arguments or settings file.
    ///   2. It may be combined with other presets.
    ///   3. Presets are a convenient way to quickly perform complex tasks. You may modify preset's output dir and plugin name(or anything else) in settings file. Though it's better to create similar "--merge" list in settings file with your own rules.
    #[arg(
        help_heading = "Presets",
        conflicts_with = "settings_write",
        short = 'T',
        long,
        alias = "preset_turn_normal_grass",
        help = "Turn Normal Grass and Kelp into Groundcover for the whole load order",
        verbatim_doc_comment
    )]
    pub(super) preset_turn_normal_grass: bool,
    /// Merge the whole load order. The pinnacle of the program.
    ///
    /// This preset is used to:
    ///   1. Find game config file
    ///   2. Get load order from it
    ///   3. Merge plugins and report missing references.
    ///   4. Produce merged plugin "MergedLoadOrder.esp".
    ///   4.1. (combined with preset-turn-normal-grass) Produce and merge -CONTENT into "MergedLoadOrder.esp".
    ///   5. (if you use grass) Produce merged grass plugin "MergedLoadOrderGrass.esp".
    ///   5.1. (combined with preset-turn-normal-grass) Produce and merge -GROUNDCOVER into "MergedLoadOrderGrass.esp".
    ///
    /// Provide path to a game config file if program fails to find one. See --config.
    ///
    /// It's a convenience alias to the following --merge strings:
    ///   -m "MergedLoadOrder.esp, no_dry_run, use_load_order, exclude_deleted_records, complete_replace, strip_masters, ignore_errors, no_insufficient_merge"
    ///      (when combined with preset-check-references):
    ///         - "show_missing_refs" is added
    ///      (when combined with preset-turn-normal-grass):
    ///         - "turn_normal_grass, no_dry_run_secondary" is added
    ///         - a bit of program logic to remove -CONTENT instances straight from the merged plugin
    ///      (when you have more than 1 grass plugin)
    ///   -m "MergedLoadOrderGrass.esp, use_load_order, exclude_deleted_records, grass, strip_masters, ignore_errors, insufficient_merge"
    ///      (when combined with preset-turn-normal-grass):
    ///         - "append_to_use_load_order:MergedLoadOrder-GROUNDCOVER.esp" is added
    ///
    /// This preset may be used for different purposes. For example make all-at-once cleaning of clipping grass with "The LawnMower for Morrowind" by acidzebra. Make merged plugin and grass, the pass them to the LawnMower to get the desired result for your whole load order.
    ///
    /// As a preset:
    ///   1. It will ignore other "--merge" lists defined via command line arguments or settings file.
    ///   2. It may be combined with other presets.
    ///   3. Presets are a convenient way to quickly perform complex tasks. You may modify preset's output dir and plugin name(or anything else) in settings file. Though it's better to create similar "--merge" list in settings file with your own rules.
    #[arg(
        help_heading = "Presets",
        conflicts_with = "settings_write",
        short = 'O',
        long,
        alias = "preset_merge_load_order",
        help = "Merge the whole load order",
        verbatim_doc_comment
    )]
    pub(super) preset_merge_load_order: bool,
    /// Mode defines how to process possibly mergeable record. Available modes are:
    ///
    ///   "keep"
    ///     - All possibly mergeable records are stacked in the output plugin, so that most record merging utilities(TES3Merge, Merged Lands etc) would be able to do their work as if plugins were not merged together. Nothing would break if you don't use any record merging utilities at all.
    ///
    ///   "keep_without_lands"
    ///     - Same as "keep", but LAND records(landscape) would simply be replaced. You may use this mode if you don't intend to use "Merged Lands".
    ///
    ///   "replace"
    ///     - All records are replaced(hence no records to merge, that's how Morrowind works with records), except leveled lists. Most leveled list merging utilities(tes3cmd, OMWLLF, Jobasha, TES3Merge etc) would be able to do their work as if plugins were not merged together. You may use this mode if you don't merge any records except leveled lists. Engine processes merged plugins of "keep" and "replace" modes exactly the same, but "replace" produces slightly smaller results.
    ///
    ///   "complete_replace"
    ///     - Same as replace, but leveled lists are replaced too. You may use this mode if you don't merge any records. I'd only recommend this mode for minimalistic mod setups or whole load order merges.
    ///
    ///   "grass"
    ///     - Same as replace, but designed for grass. Allows excluding instances that you don't like. By default it excludes "UNKNOWN GRASS" records from Remiros' Groundcover. Check "advanced.grass_filter" option in settings file if you want to exclude anything else(i.e. mushrooms).
    ///     - This mode automatically excludes non-grass statics, interior or empty cells.
    ///     - This mode implicitly sets --insufficient_merge, thus only scans for cell and static records.
    ///     - This mode implicitly sets --no-turn-normal-grass.
    ///
    /// Note about DeltaPlugin:
    ///   DeltaPlugin processes records the same way as both engines, e.g. discards different variants of mergeable records except the last one. Possible way to use both utilities is to make additional openmw.cfg file with paths to unmerged plugins, then run "delta_plugin -c openmw.cfg", then run "habasi".
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
    pub(super) mode: Option<String>,
    /// Base directory for plugin lists.
    ///
    /// By default program uses current directory("") as a base for plugin's relative paths. Plugin's absolute paths ignore this option.
    ///
    /// Examples(same result, second example show per list option instead of global):
    ///   -B "mods/Patches/BTBGIsation/03 Modular - Secondary" -m "BTBGIsation - Custom Merged.esp, BTBGIsation - Magical Missions.esp, BTBGIsation - Weapons Expansion Morrowind.esp"
    ///   -m "BTBGIsation - Custom Merged.esp, base_dir:mods/Patches/BTBGIsation/03 Modular - Secondary, BTBGIsation - Magical Missions.esp, BTBGIsation - Weapons Expansion Morrowind.esp"
    ///
    /// Default value: "".
    ///
    /// Corresponding per list option: "base_dir:<PATH>", default value: "base_dir:"(empty, e.g. current directory).
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short,
        long,
        alias = "base_dir",
        help = "Base directory for plugin lists",
        value_name = "PATH",
        verbatim_doc_comment
    )]
    pub(super) base_dir: Option<String>,
    /// Do not write output plugin.
    ///
    /// Corresponding per list options: "dry_run", "no_dry_run".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short,
        long,
        alias = "dry_run",
        help = "Do not write output plugin"
    )]
    pub(super) dry_run: bool,
    /// Use plugins list from game config file, replacing list of plugins defined in list(if any). Program tries to automatically find config, though sometimes it should be pointed to it with --config. It will report game config file errors(missing plugins, missing directories).
    ///
    /// This option implicitly sets --base-dir to ""(empty).
    ///
    /// Corresponding per list options: "use_load_order", "no_use_load_order".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short = 'u',
        long,
        alias = "use_load_order",
        help = "Use plugins list from game config file"
    )]
    pub(super) use_load_order: bool,
    /// Path to the game config file, e.g.: "C:\Users\Username\Documents\My Games\OpenMW\openmw.cfg"(absolute), "../Morrowind.ini"(relative). May be used to provide alternative game config file or in case the game config file was not found automatically.
    ///
    /// Some options(--turn-normal-grass) require list of mods directories to scan for meshes. OpenMW's game config file contains it. Morrowind's "classic" approach also has it(Data Files dir with all the mods dumped into). Morrowind with Mod Organizer and mods stored in different directories doesn't provide this list. Use "ModOrganizer-to-OpenMW" MO plugin to create openmw.cfg and point to it.
    ///
    /// Some presets(--preset-merge-load-order) also scan for grass plugins. OpenMW's game config file contains it. Morrowind with Mod Organizer is covered too(with help of previously mentioned "ModOrganizer-to-OpenMW" MO plugin). Morrowind's "classic" approach is out of luck, because there is no grass section in Morrowind.ini.
    ///
    /// Default value: ""(automatically search for the game config file).
    ///
    /// Corresponding per list option: "config:<PATH>", default value: "config:"(automatically search for the game config file).
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short,
        long,
        value_name = "PATH",
        value_hint = clap::ValueHint::FilePath,
        help = "Path to the game config file"
    )]
    pub(super) config: Option<String>,
    /// Show all missing references.
    ///
    /// By default only first missing reference per cell is logged to prevent noise.
    ///
    /// Corresponding per list options: "show_all_missing_refs", "no_show_all_missing_refs".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short = 'a',
        long,
        alias = "show_all_missing_refs",
        help = "Show all missing references"
    )]
    pub(super) show_all_missing_refs: bool,
    /// Turn Normal Grass and Kelp into Groundcover. Idea, name of the option and list of statics(reversed from list of meshes) are taken from the same name original mod and "Stirk Performance Improver" by Hemaris.
    ///
    /// It's possible to recreate original Hemaris' mods(example comes further), but the goal is to provide easy-to-use automatic way to make the same thing for any plugin or list of plugins. See --preset-turn-normal-grass to quickly make it for your whole load order. Check original mod's description for more details.
    ///
    /// This option is used to:
    ///   1. Find game config file
    ///   2. Get load order, list of plugins, BSAs and directories from it
    ///   3. Scan plugin cells for statics that may be converted to groundcover
    ///   4. Scan and retrieve appropriate meshes from loose files or BSAs
    ///   5. Produce 2 additional output plugins:
    ///     - "<OUTPUT_PLUGIN_NAME>-CONTENT.esp" with deleted references: load it as a normal plugin in the end of load order
    ///     - "<OUTPUT_PLUGIN_NAME>-GROUNDCOVER.esp" with new grass: load it as a groundcover plugin
    ///     - meshes folder with grass meshes(same meshes that were used for "normal" grass in your setup): these meshes will be used by groundcover plugin
    ///
    /// Provide path to a game config file if program fails to find one. See --config.
    ///
    /// By default this option mimics engine's behaviour. It will select younger mesh(comparing date of loose file and BSA) when mesh with the same name exists as loose file and in BSA(loose files are younger in most cases). You may opt out of this behaviout with --prefer_loose_over_bsa.
    ///
    /// Example 1:
    ///   --turn-normal-grass --dry-run -m "recreate_vvardenfell/hm-grass-vvardenfell.esp,Morrowind.esm" -m "recreate_mainland/hm-grass-mainland.esp,TR_Mainland.esm" -m "recreate_stirk/hm-grass-stirk.esp,Cyr_Main.esm"
    ///
    ///   First two "-m" in example recreate original Hemaris' mod. Third "-m" recreates "Stirk Performance Improver". Option --dry-run is used because we don't need merged plugins themselves. As a result three directories will be created with contents similar or exactly the same as in the original mods(Morrowind Optimization Patch would better be in load order for maximum similarity because original mod used meshes from it).
    ///
    /// Example 2:
    ///   -m "recreate_all_with_totsp_and_fixes/hm-grass-all-with-totsp-and-fixes.esp, turn_normal_grass, dry_run, Morrowind.esm, Tribunal.esm, Bloodmoon.esm, Patch for Purists.esm, TR_Mainland.esm, Sky_Main.esm, Cyr_Main.esm, Solstheim Tomb of The Snow Prince.esm, TR_Hotfix.esp"
    ///
    ///   Almost the same as first example. The difference is that it produces less files, adds whole base game and STOTSP, incorporates fixes from PFP and TR_Hotfix.
    ///
    /// This option automatically excludes previously created -CONTENT plugin placed into the load order.
    ///
    /// It's possible to modify list of statics in settings file's "advanced.turn_normal_grass_stat_ids". Uncomment the list, then add new or comment out the statics you don't want to be turned to groundcover. For example I've noticed that "flora_bc_grass_01" may look weird with some weather when turned into groundcover.
    ///
    /// Corresponding per list options: "turn_normal_grass", "no_turn_normal_grass".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short = 't',
        long,
        alias = "turn_normal_grass",
        help = "Turn Normal Grass and Kelp into Groundcover",
        verbatim_doc_comment
    )]
    pub(super) turn_normal_grass: bool,
    /// Get mesh from BSA only when loose mesh not available. Only effective with --turn-normal-grass. By default younger mesh is used if it's available both in BSA and as a loose file(engine's behaviour, loose files are selected in most cases).
    ///
    /// Corresponding per list options: "prefer_loose_over_bsa", "no_prefer_loose_over_bsa".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short = 'p',
        long,
        alias = "prefer_loose_over_bsa",
        help = "Get mesh from BSA only when loose mesh not available"
    )]
    pub(super) prefer_loose_over_bsa: bool,
    /// Reindex references twice.
    ///
    /// References are numbered as they appear by default. Cells would contain non-continious ranges of reference ids as a result. Use this option to reindex references again at the expense of additional processing time(up to 30%). This option doesn't change anything gameplay wise, it only makes output plugin internally look like it was produced by TES-CS.
    ///
    /// Corresponding per list options: "reindex", "no_reindex".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short = 'r',
        long,
        help = "Reindex references twice"
    )]
    pub(super) reindex: bool,
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
        alias = "strip_masters",
        help = "Strip masters when possible"
    )]
    pub(super) strip_masters: bool,
    /// Exclude deleted records with --use-load-order.
    ///
    /// Records with DELETED flag are excluded from the output plugin with this option.
    ///
    /// Combination with --turn-normal-grass option also leads to -CONTENT plugin not being created. References that'd be listed in -CONTENT are deleted from the result plugin instead.
    ///
    /// This option implicitly sets --use-load-order because it makes not sense otherwise.
    ///
    /// Corresponding per list options: "exclude_deleted_records", "no_exclude_deleted_records".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short = 'E',
        long,
        alias = "exclude_deleted_records",
        help = "Exclude deleted records with --use-load-order"
    )]
    pub(super) exclude_deleted_records: bool,
    /// Do not show missing references.
    ///
    /// This option takes precedence when used together with --show-all-missing-refs.
    ///
    /// Corresponding per list options: "show_missing_refs", "no_show_missing_refs".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short = 'A',
        long,
        alias = "no_show_missing_refs",
        help = "Do not show missing references"
    )]
    pub(super) no_show_missing_refs: bool,
    /// All versions of record would be placed into the output plugin. May be useful for investigating record mutations.
    ///
    /// Corresponding per list options: "debug", "no_debug".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short = 'D',
        long,
        help = "Enable additional debug mode"
    )]
    pub(super) debug: bool,
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
        alias = "no_ignore_errors",
        help = "Do not ignore non-important errors"
    )]
    pub(super) no_ignore_errors: bool,
    /// Do not compare output plugin with previous version if it exists.
    ///
    /// By default program doesn't overwrite previous version of output plugin if it's not changed. Disabling comparison would slightly improve processing time.
    ///
    /// Corresponding per list options: "no_compare", "compare".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        short = 'P',
        long,
        alias = "no_compare",
        help = "Do not compare output plugin with previous version"
    )]
    pub(super) no_compare: bool,
    /// Do not compare secondary output plugin with previous version if it exists.
    ///
    /// Some options(see --turn-normal-grass) may produce "secondary" output plugins in addition to the "primary" merged plugin. By default program doesn't overwrite previous version of output plugin if it's not changed. Disabling comparison would slightly improve processing time.
    ///
    /// Corresponding per list options: "no_compare_secondary", "compare_secondary".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        long,
        alias = "no_compare_secondary",
        help = "Do not compare output secondary plugin with previous version"
    )]
    pub(super) no_compare_secondary: bool,
    /// Do not write secondary output plugin.
    ///
    /// Some options(see --turn-normal-grass) may produce "secondary" output plugins in addition to the "primary" merged plugin.
    ///
    /// Corresponding per list options: "dry_run_secondary", "no_dry_run_secondary".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        long,
        alias = "dry_run_secondary",
        help = "Do not write secondary output plugin"
    )]
    pub(super) dry_run_secondary: bool,
    /// Dismiss stats with --dry-run.
    ///
    /// It's made specifically for -C and -T presets, which are designed not to produce "primary" file. It's hardly needed for anything else, but I've tried to make presets as transparent and reproducible as possible.
    ///
    /// Corresponding per list options: "dry_run_dismiss_stats", "no_dry_run_dismiss_stats".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        long,
        alias = "dry_run_dismiss_stats",
        help = "Dismiss stats with --dry-run"
    )]
    pub(super) dry_run_dismiss_stats: bool,
    /// Turn glob/regex patterns to case-sensitive mode.
    ///
    /// By default glob/regex patterns are case-insensitive.
    ///
    /// Corresponding per list options: "regex_case_sensitive", "no_regex_case_sensitive".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        long,
        alias = "regex_case_sensitive",
        help = "Turn glob/regex patterns to case-sensitive mode"
    )]
    pub(super) regex_case_sensitive: bool,
    /// Sort plugins from glob/regex patterns by name.
    ///
    /// By default plugins from glob/regex patterns are sorted by modification time. Sorting by name is used by default when modification time is unavailable.
    ///
    /// Corresponding per list options: "regex_sort_by_name", "no_regex_sort_by_name".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        long,
        alias = "regex_sort_by_name",
        help = "Sort plugins from glob/regex patterns by name"
    )]
    pub(super) regex_sort_by_name: bool,
    /// Ignore non-critical errors, e.g. missing or broken plugins.
    ///
    /// It may be useful, though it's better to fix underlying problems. May rarely lead to unexpected behaviour.
    ///
    /// Corresponding per list options: "ignore_important_errors", "no_ignore_important_errors".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        long,
        alias = "ignore_important_errors",
        help = "Ignore non-critical errors"
    )]
    pub(super) ignore_important_errors: bool,
    /// Process only cell references(and statics with '-M grass' or '-t').
    ///
    /// This option improves performance a bit, allows filtering out unneeded records with grass mode. Should obviously be used with care. It's made specifically for grass mode and presets -C, -T.
    ///
    /// Corresponding per list options: "insufficient_merge", "no_insufficient_merge".
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        long,
        alias = "insufficient_merge",
        help = "Process only cell references(and statics with '-M grass' or '-t')"
    )]
    pub(super) insufficient_merge: bool,
    /// Append plugin path to --use-load-order list. This option would only be effective combined with --use-load-order.
    ///
    /// It's made specifically for combination of -O and -T presets to allow adding newly created -GROUNDCOVER plugin into groundcover plugins list. May probably be used for similar tasks. Similar to --skip-from-use-load-order, though requires path to plugin instead of plugin name.
    ///
    /// Default value: ""(option turned off).
    ///
    /// Corresponding per list option: "append_to_use_load_order:<PATH>", default value: "append_to_use_load_order:"(option turned off).
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        long,
        alias = "append_to_use_load_order",
        value_name = "PATH",
        help = "Append plugin path to --use-load-order list"
    )]
    pub(super) append_to_use_load_order: Option<String>,
    /// Skip plugin name from --use-load-order list. This option would only be effective combined with --use-load-order.
    ///
    /// It's made specifically for combination of -O and -T presets to allow skipping -CONTENT plugin made with another plugin list. May probably be used for similar tasks. Similar to --append-to-use-load-order, though requires plugin name instead of path to plugin.
    ///
    /// Default value: ""(option turned off).
    ///
    /// Corresponding per list option: "skip_from_use_load_order:<NAME>", default value: "skip_from_use_load_order:"(option turned off).
    #[arg(
        help_heading = "List options",
        conflicts_with = "settings_write",
        long,
        alias = "skip_from_use_load_order",
        value_name = "NAME",
        help = "Skip plugin name from --use-load-order list"
    )]
    pub(super) skip_from_use_load_order: Option<String>,
    /// Show more information. May be provided multiple times for extra effect:
    ///
    ///   -v: Show list options, total stats per list, list of new grass meshes written, count of new static records, list of records excluded(with exclude_deleted_records option), "references reindexed" and "master subrecords stripped" messages.
    ///
    ///   -vv: Show detailed total stats, ignored reference errors, list of new grass meshes untouched, list of new static records, list of records excluded(grass mode, INFO records from settings.advanced.keep_only_last_info_ids),"processing plugin" messages.
    ///
    ///   -vvv: Show detailed list stats, previous log file backup message, failed guesses of new grass meshes filenames.
    ///
    /// Flag --quiet takes precedence over this flag.
    #[arg(
        help_heading = "Display output",
        short,
        long,
        action = clap::ArgAction::Count,
        help = "Show more information",
        verbatim_doc_comment,
    )]
    pub(super) verbose: u8,
    /// Do not show anything.
    ///
    /// This flag takes precedence over --verbose.
    #[arg(
        help_heading = "Display output",
        short,
        long,
        help = "Do not show anything"
    )]
    pub(super) quiet: bool,
    /// Show list of plugins to merge(handy when using wildcard/glob/regex patterns).
    #[arg(
        help_heading = "Display output",
        short = 'w',
        long,
        help = "Show list of plugins to merge(handy when using wildcard/glob/regex patterns)"
    )]
    pub(super) show_plugins: bool,
}

fn arg_get_help(arg: &Arg) -> Result<StyledStr> {
    arg.get_long_help().map_or_else(
        || {
            arg.get_help().map_or_else(
                || {
                    Err(anyhow!(
                        "Error: failed to find help for \"{}\" argument",
                        arg.get_id()
                    ))
                },
                |help| Ok(help.clone()),
            )
        },
        |help| Ok(help.clone()),
    )
}

fn check_long_arg_names_and_aliases(string_raw: &str, command: &clap::Command) -> Result<()> {
    let mut string = string_raw.to_lowercase().replace('-', "_");
    if let Some(stripped) = string.strip_prefix("__") {
        string = stripped.to_owned();
    }
    match string.as_ref() {
        "help" => return Err(anyhow!("Print help (see a summary with '-h')")),
        "version" => return Err(anyhow!("Print version")),
        _ => {
            for arg in command.get_arguments() {
                if arg.get_id() == &string {
                    return Err(anyhow!(arg_get_help(arg)?));
                } else if let Some(vec) = arg.get_all_aliases() {
                    for alias in vec {
                        if alias.to_lowercase().replace('-', "_") == string {
                            return Err(anyhow!(arg_get_help(arg)?));
                        }
                    }
                } else { //
                }
            }
        }
    };
    Ok(())
}

fn check_short_arg_names_and_aliases(string_raw: &str, command: &clap::Command) -> Result<()> {
    let string = string_raw
        .strip_prefix('-')
        .map_or_else(|| string_raw.to_owned(), ToOwned::to_owned);
    if string.len() == 1 {
        let character = string.chars().next().context("string is empty")?;
        match character {
            'h' => return Err(anyhow!("Print help (see more with '--help')")),
            'V' => return Err(anyhow!("Print version")),
            _ => {
                for arg in command.get_arguments() {
                    if let Some(short) = arg.get_short() {
                        if short == character {
                            return Err(anyhow!(arg_get_help(arg)?));
                        }
                    };
                    if let Some(vec) = arg.get_all_short_aliases() {
                        for alias in vec {
                            if alias == character {
                                return Err(anyhow!(arg_get_help(arg)?));
                            }
                        }
                    }
                }
            }
        }
    };
    Ok(())
}

fn check_show_help_for_option(options: &Options) -> Result<()> {
    if let Some(ref string) = options.help_option {
        let command = Options::command();
        check_long_arg_names_and_aliases(string, &command)?;
        check_short_arg_names_and_aliases(string, &command)?;
        Err(anyhow!(
            "Failed to find option \"{}\" to show help for it. Use \"-h\" to get list of available options.",
            string
        ))
    } else {
        Ok(())
    }
}

pub(in crate::config) fn get_options() -> Result<Options> {
    let options = Options::try_parse_from(wild::args_os())?;
    check_show_help_for_option(&options)?;
    Ok(options)
}
