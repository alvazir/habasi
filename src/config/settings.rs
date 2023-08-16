use super::{check_settings_version, SettingsFile};
use anyhow::{Context, Result};
use confique::Config;

#[derive(Config)]
pub(crate) struct Settings {
    #[config(nested)]
    pub(crate) options: Options,
    #[config(nested)]
    pub(crate) advanced: Advanced,
    #[config(nested)]
    pub(crate) guts: Guts,
}

#[derive(Config)]
pub(crate) struct Options {
    /// Description of all the options is provided with --help-option <OPTION> or --help. There are two lines per each option: default value and set value. Uncomment second line for the needed option and set the value.
    ///
    /// [merge] The heart of the program. This is is the only option you need in most cases. Examples:
    ///
    /// 1. Multiple merged plugins(shows both "one-line" and "multi-line" styles, the latter is more suitable for longer lists):
    ///
    ///   merge = [
    ///   ["MergedGhostRevenge.esp", "GhostRevenge.ESP", "GhostRevenge_TR1912.esp"],
    ///   [
    ///   "MergedPlugin2.esp",
    ///   "replace",
    ///   "Frozen in Time.esp",
    ///   "The Minotaurs Ring.esp",
    ///   "Cure all the Kwama Queens.ESP",
    ///   ],
    ///   ]
    ///
    /// 2. You may process only one plugin to throw away some redundant (sub)records. Plugin may become slimer, though it's not the same as tes3cmd's plugin cleaning. Best example would be "Sky_Main_Grass.esp" from SHotN v23.02(25MiB to 13MiB size, 653 to 287 records while providing the same content):
    ///
    ///   merge = [["Sky_Grass_Slim.esp, grass, Sky_Main_Grass.esp"]]
    ///
    /// 3. Different styles of providing Windows paths:
    ///
    ///   merge = [["United-100.esp", "C:/Data Files/Morrowind.esm", "C:\\Data Files\\Tribunal.esm", 'C:\Data Files\Bloodmoon.esm', '''C:\Data Files\mod with quote' in name.esp''']]
    ///
    /// [Per list options] First value listed is default when global list option is not set.
    ///   - [mode] "keep", "keep_without_lands", "replace", "complete_replace", "grass"
    ///   - [base_dir] "base_dir:", "base_dir:<PATH>"
    ///   - [dry_run] "no_dry_run", "dry_run"
    ///   - [use_load_order] "no_use_load_order", "use_load_order"
    ///   - [config] "config:", "config:<PATH>"
    ///   - [show_all_missing_refs] "no_show_all_missing_refs", "show_all_missing_refs"
    ///   - [turn_normal_grass] "no_turn_normal_grass", "turn_normal_grass"
    ///   - [prefer_loose_over_bsa] "no_prefer_loose_over_bsa", "prefer_loose_over_bsa"
    ///   - [reindex] "no_reindex", "reindex"
    ///   - [strip_masters] "no_strip_masters", "strip_masters"
    ///   - [exclude_deleted_records] "no_exclude_deleted_records", "exclude_deleted_records"
    ///   - [no_show_missing_refs] "show_missing_refs", "no_show_missing_refs"
    ///   - [debug] "no_debug", "debug"
    ///   - [no_ignore_errors] "ignore_errors", "no_ignore_errors"
    ///   - [no_compare] "compare", "no_compare"
    ///   - [no_compare_secondary] "compare_secondary", "no_compare_secondary"
    ///   - [dry_run_secondary] "no_dry_run_secondary", "dry_run_secondary".
    ///   - [dry_run_dismiss_stats] "no_dry_run_dismiss_stats", "dry_run_dismiss_stats".
    ///   - [ignore_important_errors] "no_ignore_important_errors", "ignore_important_errors".
    ///   - [insufficient_merge] "no_insufficient_merge", "insufficient_merge".
    ///   - [append_to_use_load_order] "append_to_use_load_order:", "append_to_use_load_order:<PATH>"
    ///   - [skip_from_use_load_order] "skip_from_use_load_order:", "skip_from_use_load_order:<NAME>"
    #[config(default = [])]
    pub(crate) merge: Vec<Vec<String>>,
    #[config(default = "")]
    pub(crate) log: String,
    #[config(default = false)]
    pub(crate) no_log: bool,
    #[config(default = true)]
    pub(crate) grass: bool,
    /// [--verbosity] Number corresponds to the number of verbose flags passed, e.g. -v = 1, -vv = 2, -vvv = 3
    #[config(default = 0)]
    pub(crate) verbose: u8,
    #[config(default = false)]
    pub(crate) quiet: bool,
    /// [Presets] Enabled preset ignores --merge options provided via command line or settings file. Do not enable(set to true) presets unless that's the only thing you need from the program.
    #[config(default = false)]
    pub(crate) preset_check_references: bool,
    #[config(default = false)]
    pub(crate) preset_turn_normal_grass: bool,
    #[config(default = false)]
    pub(crate) preset_merge_load_order: bool,
    /// [Global list options] Global list options are used when there is no per list options provided via "merge" section in settings file or "--merge" command line argument. Per list options take precedence over global list options for the list.
    #[config(default = "keep")]
    pub(crate) mode: String,
    #[config(default = "")]
    pub(crate) base_dir: String,
    #[config(default = false)]
    pub(crate) dry_run: bool,
    #[config(default = false)]
    pub(crate) use_load_order: bool,
    #[config(default = "")]
    pub(crate) config: String,
    #[config(default = false)]
    pub(crate) show_all_missing_refs: bool,
    #[config(default = false)]
    pub(crate) turn_normal_grass: bool,
    #[config(default = false)]
    pub(crate) prefer_loose_over_bsa: bool,
    #[config(default = false)]
    pub(crate) reindex: bool,
    #[config(default = false)]
    pub(crate) strip_masters: bool,
    #[config(default = false)]
    pub(crate) exclude_deleted_records: bool,
    #[config(default = false)]
    pub(crate) no_show_missing_refs: bool,
    #[config(default = false)]
    pub(crate) debug: bool,
    #[config(default = false)]
    pub(crate) no_ignore_errors: bool,
    #[config(default = false)]
    pub(crate) no_compare: bool,
    #[config(default = false)]
    pub(crate) no_compare_secondary: bool,
    #[config(default = false)]
    pub(crate) dry_run_secondary: bool,
    #[config(default = false)]
    pub(crate) dry_run_dismiss_stats: bool,
    #[config(default = false)]
    pub(crate) ignore_important_errors: bool,
    #[config(default = false)]
    pub(crate) insufficient_merge: bool,
    #[config(default = "")]
    pub(crate) append_to_use_load_order: String,
    #[config(default = "")]
    pub(crate) skip_from_use_load_order: String,
}

#[derive(Config)]
pub(crate) struct Advanced {
    /// [grass_filter] This filter works only in "grass" mode. By default it filters out "UNKNOWN_GRASS" records from Remiros Groundcover. It's possible to filter more by adding to the list(i.e. if you don't like some kind of grass or added mushrooms etc). Values are case insensitive.
    #[config(default = ["unknown_grass"])]
    pub(crate) grass_filter: Vec<String>,
    /// [turn_normal_grass_stat_ids] List of static IDs that are used with turn_normal_grass option. Each record format is "<Fallback_plugin(where static was introduced)>:<Static_name(case_insensitive)>".
    #[config(default = [
"Morrowind.esm:Flora_Ash_Grass_R_01",
"Morrowind.esm:Flora_BC_Lilypad",
"Morrowind.esm:Flora_kelp_01",
"Morrowind.esm:Flora_kelp_02",
"Morrowind.esm:Flora_kelp_03",
"Morrowind.esm:Flora_kelp_04",
"Morrowind.esm:flora_ash_grass_b_01",
"Morrowind.esm:flora_ash_grass_w_01",
"Morrowind.esm:flora_bc_fern_02",
"Morrowind.esm:flora_bc_fern_03",
"Morrowind.esm:flora_bc_fern_04",
"Morrowind.esm:flora_bc_grass_01",
"Morrowind.esm:flora_bc_grass_02",
"Morrowind.esm:flora_bc_lilypad_02",
"Morrowind.esm:flora_bc_lilypad_03",
"Morrowind.esm:flora_grass_01",
"Morrowind.esm:flora_grass_02",
"Morrowind.esm:flora_grass_03",
"Morrowind.esm:flora_grass_04",
"Morrowind.esm:in_cave_plant00",
"Morrowind.esm:in_cave_plant10",
"Tribunal.esm:Flora_grass_05",
"Tribunal.esm:Flora_grass_06",
"Tribunal.esm:Flora_grass_07",
"Bloodmoon.esm:Flora_BM_grass_01",
"Bloodmoon.esm:Flora_BM_grass_02",
"Bloodmoon.esm:Flora_BM_grass_03",
"Bloodmoon.esm:Flora_BM_grass_04",
"Bloodmoon.esm:Flora_BM_grass_05",
"Bloodmoon.esm:Flora_BM_grass_06",
"Bloodmoon.esm:Flora_BM_shrub_01",
"Bloodmoon.esm:Flora_BM_shrub_02",
"Bloodmoon.esm:Flora_BM_shrub_03",
"Tamriel_Data.esm:T_Glb_Flora_Fern_01",
"Tamriel_Data.esm:T_Glb_Flora_Fern_02",
"Tamriel_Data.esm:T_Mw_FloraAT_LilypadOrange_01",
"Tamriel_Data.esm:T_Mw_FloraAT_LilypadOrange_02",
"Tamriel_Data.esm:T_Mw_FloraAT_LilypadOrange_03",
"Tamriel_Data.esm:T_Mw_FloraAT_SpartiumBealei_01",
"Tamriel_Data.esm:T_Mw_FloraAT_SpartiumBealei_02",
"Tamriel_Data.esm:T_Mw_FloraAT_SpartiumBealei_03",
// PC stats
"Tamriel_Data.esm:T_Glb_Flora_Cattails_01",
"Tamriel_Data.esm:T_Glb_Flora_Cattails_02",
"Tamriel_Data.esm:T_Glb_Flora_Cattails_03",
"Tamriel_Data.esm:T_Cyr_FloraGC_Bush_02",
"Tamriel_Data.esm:T_Cyr_FloraGC_Shrub_01",
"Tamriel_Data.esm:T_Cyr_FloraGC_Shrub_02",
"Tamriel_Data.esm:T_Cyr_Flora_Lilypad_01",
"Tamriel_Data.esm:T_Cyr_Flora_Lilypad_02",
"Tamriel_Data.esm:T_Cyr_Flora_Lilypad_03",
"Tamriel_Data.esm:T_Cyr_FloraStr_Shrub_01",
// no longer used it seems
// "Tamriel_Data.esm:T_Glb_Flora_WtHyacinth_01",
// "Tamriel_Data.esm:T_Glb_Flora_WtHyacinth_02",
// "Tamriel_Data.esm:T_Glb_Flora_WtHyacinth_03",
// "Tamriel_Data.esm:T_Glb_Flora_WtHyacinth_04",
    ])]
    pub(crate) turn_normal_grass_stat_ids: Vec<String>,
    /// [keep_only_last_info_ids] Previous instance of the INFO record is removed when record with the same ID(and from the same topic) comes into a merged plugin. Format: ["ID", "Topic(case insensitive)", "Reason"].
    #[config(default = [["19511310302976825065", "threaten", "This record is problematic when coming from both LGNPC_GnaarMok and LGNPC_SecretMasters. I've failed to find the reason. Error in OpenMW-CS: \"Loading failed: attempt to change the ID of a record\"."]])]
    pub(crate) keep_only_last_info_ids: Vec<Vec<String>>,
}

#[derive(Config)]
pub(crate) struct Guts {
    /// Guts of the program. Use at your own risk ;-)
    ///
    /// # Following line is used to determine version of used settings to warn about outdated version:
    /// # Settings version: 0.2.5
    ///
    /// [Section: Presets]
    #[config(default = ["CheckReferences.esp", "dry_run", "use_load_order", "show_missing_refs", "complete_replace", "no_compare", "ignore_errors", "insufficient_merge", "dry_run_dismiss_stats"])]
    pub(crate) preset_config_check_references: Vec<String>,
    #[config(default = ["TurnNormalGrass.esp", "dry_run", "use_load_order", "turn_normal_grass", "complete_replace", "no_compare", "ignore_errors", "insufficient_merge", "dry_run_dismiss_stats", "no_show_missing_refs"])]
    pub(crate) preset_config_turn_normal_grass: Vec<String>,
    #[config(default = ["show_missing_refs"])]
    pub(crate) preset_config_turn_normal_grass_add_with_check_references: Vec<String>,
    #[config(default = ["MergedLoadOrder.esp", "no_dry_run", "use_load_order", "exclude_deleted_records", "complete_replace", "strip_masters", "ignore_errors", "no_insufficient_merge"])]
    pub(crate) preset_config_merge_load_order: Vec<String>,
    #[config(default = ["MergedLoadOrderGrass.esp", "use_load_order", "exclude_deleted_records", "grass", "strip_masters", "ignore_errors", "insufficient_merge"])]
    pub(crate) preset_config_merge_load_order_grass: Vec<String>,
    #[config(default = ["show_missing_refs"])]
    pub(crate) preset_config_merge_load_order_add_with_check_references: Vec<String>,
    #[config(default = ["turn_normal_grass", "no_dry_run_secondary"])]
    pub(crate) preset_config_merge_load_order_add_with_turn_normal_grass: Vec<String>,
    /// [Section: Game configuration file] For both Morrowind.ini and openmw.cfg. Actually file may have any name or extension. "/" is used as separator for all platforms.
    ///
    /// Path that is appended to the "preference_dir": "$HOME/.config|$HOME/Library/Preferences" + config_path_suffix_linux_macos
    #[config(default = "openmw/openmw.cfg")]
    pub(crate) config_path_suffix_linux_macos: String,
    /// Path that is appended to the "document_dir": "C:\Users\Username\Documents" + config_path_suffix_windows
    #[config(default = "My Games/OpenMW/openmw.cfg")]
    pub(crate) config_path_suffix_windows: String,
    /// All other relative/absolute paths to check:
    ///  "/storage/emulated/0/omw/config/openmw.cfg": android openmw.cfg absolute path
    ///  "openmw.cfg": all platforms, looks for openmw.cfg in the directory where it's run
    ///  "Morrowind.ini": all platforms, looks for Morrowind.ini in the directory where it's run
    ///  "../Morrowind.ini": all platforms, looks for Morrowind.ini in the parent directory from where it's run(e.g. "Data Files")
    #[config(default = ["/storage/emulated/0/omw/config/openmw.cfg", "openmw.cfg", "Morrowind.ini", "../Morrowind.ini"])]
    pub(crate) config_paths_list: Vec<String>,
    #[config(default = "GameFile")]
    pub(crate) mor_line_beginning_content: String,
    #[config(default = "Archive")]
    pub(crate) mor_line_beginning_archive: String,
    /// Morrowind.bsa is not listed in Morrowind.ini, though it is needed with some options.
    #[config(default = "Archive=Morrowind.bsa")]
    pub(crate) mor_line_missing_archive: String,
    #[config(default = "Data Files")]
    pub(crate) mor_data_files_dir: String,
    #[config(default = "content=")]
    pub(crate) omw_line_beginning_content: String,
    #[config(default = "data=")]
    pub(crate) omw_line_beginning_data: String,
    #[config(default = "fallback-archive=")]
    pub(crate) omw_line_beginning_fallback_archive: String,
    #[config(default = "groundcover=")]
    pub(crate) omw_line_beginning_groundcover: String,
    #[config(default = ["esm", "esp", "omwaddon", "bsa"])]
    pub(crate) omw_plugin_extensions: Vec<String>,
    /// [Section: Turn normal grass]
    #[config(default = ":")]
    pub(crate) turn_normal_grass_stat_ids_separator: String,
    #[config(default = 100)]
    pub(crate) turn_normal_grass_mesh_new_name_retries: u8,
    #[config(default = "-CONTENT.esp")]
    pub(crate) turn_normal_grass_plugin_name_suffix_deleted_content: String,
    #[config(default = "-GROUNDCOVER.esp")]
    pub(crate) turn_normal_grass_plugin_name_suffix_grass: String,
    #[config(default = ", idea by Hemaris")]
    /// "\n" is the new line symbol.
    pub(crate) turn_normal_grass_header_author_append: String,
    #[config(
        default = "ENABLE THIS PLUGIN AS A NORMAL MOD.\nTurns selected plugins' grass-shaped static plants into \"grass\" in the grass mod sense."
    )]
    pub(crate) turn_normal_grass_header_description_content: String,
    #[config(
        default = "OPENMW PLAYERS: LOAD THIS WITH A GROUNDCOVER= LINE IN OPENMW.CFG.\nMGE XE USERS: ONLY ENABLE THIS WHILE GENERATING DISTANT LAND.\nTurns selected plugins' grass-shaped static plants into \"grass\" in the grass mod sense."
    )]
    pub(crate) turn_normal_grass_header_description_groundcover: String,
    /// [Section: Meshes]
    #[config(default = "nif")]
    pub(crate) mesh_extension: String,
    #[config(default = "meshes")]
    pub(crate) meshes_dir: String,
    #[config(default = "grass")]
    pub(crate) grass_subdir: String,
    /// [Section: Header] Output plugin will have these values placed into header.
    #[config(default = 1.3)]
    pub(crate) header_version: f32,
    #[config(default = "Habasi")]
    pub(crate) header_author: String,
    /// Many plugins merged would result in "Auto-merged X plugins".
    #[config(default = "Auto-merged ")]
    pub(crate) header_description_merged_many_plugins_prefix: String,
    #[config(default = " plugins")]
    pub(crate) header_description_merged_many_plugins_suffix: String,
    /// One processed plugin would result in 'Processed plugin "PLUGIN_NAME"'.
    #[config(default = "Processed plugin \"")]
    pub(crate) header_description_processed_one_plugin_prefix: String,
    #[config(default = "\"")]
    pub(crate) header_description_processed_one_plugin_suffix: String,
    /// [Section: Backup files suffixes]
    #[config(default = ".backup")]
    pub(crate) settings_backup_suffix: String,
    #[config(default = ".previous")]
    pub(crate) log_backup_suffix: String,
    /// [Section: Prefixes of per list options that take paths]
    #[config(default = "base_dir:")]
    pub(crate) list_options_prefix_base_dir: String,
    #[config(default = "config:")]
    pub(crate) list_options_prefix_config: String,
    #[config(default = "append_to_use_load_order:")]
    pub(crate) list_options_prefix_append_to_use_load_order: String,
    #[config(default = "skip_from_use_load_order:")]
    pub(crate) list_options_prefix_skip_from_use_load_order: String,
    /// [Section: Messages] Unsorted parts of messages used in multiple places.
    #[config(default = "Combined plugin lists stats:")]
    pub(crate) prefix_combined_stats: String,
    #[config(default = ". Stats:")]
    pub(crate) prefix_list_stats: String,
    #[config(default = "Ignored important error: ")]
    pub(crate) prefix_ignored_important_error_message: String,
    #[config(default = "\n\tFix the problem or add \"--ignore-important-errors\"(may rarely cause unexpected behaviour) to ignore")]
    pub(crate) suffix_add_ignore_important_errors_suggestion: String,
}

pub(crate) fn get_settings(settings_file: &mut SettingsFile) -> Result<Settings> {
    let settings = Settings::builder()
        .file(&settings_file.path)
        .load()
        .with_context(|| "Failed to load settings. Try to recreate settings file or run without it.")?;
    check_settings_version(settings_file)?;
    Ok(settings)
}
