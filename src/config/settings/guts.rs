use confique::Config;

#[allow(clippy::doc_markdown)]
#[derive(Config)]
pub struct Guts {
    /// Guts of the program. Use at your own risk ;-)
    ///
    /// # Following line is used to determine version of used settings to warn about outdated version:
    /// # Settings version: 0.3.3
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
    #[config(default = ["esm", "esp", "omwaddon", "bsa", "omwscripts"])]
    pub(crate) omw_plugin_extensions: Vec<String>,
    /// Plugins with the following extensions will not be processed. It's made to ignore .omwscripts, though may be used for anything else.
    #[config(default = ["omwscripts"])]
    pub(crate) plugin_extensions_to_ignore: Vec<String>,
    /// Plugins with the following record types not be processed. It's made to ignore plugins with non-TES3 record types newly appeared types.
    #[config(default = ["LUAL", "CELL::XSCL", "TES3::FORM"])]
    pub(crate) unexpected_tags_to_ignore: Vec<String>,
    #[config(default = 1_u8)]
    pub(crate) skipped_processing_plugins_msg_verbosity: u8,
    /// [Section: "Hidden" OpenMW-CS data directory]
    ///
    /// Path that is appended to the "data_dir": "$HOME/.local/share|$HOME/Library/Application Support" + omw_cs_data_path_suffix_linux_macos
    #[config(default = "openmw/data")]
    pub(crate) omw_cs_data_path_suffix_linux_macos: String,
    /// Path that is appended to the "document_dir": "C:\Users\Username\Documents" + omw_cs_data_path_suffix_windows
    #[config(default = "My Games/OpenMW/data")]
    pub(crate) omw_cs_data_path_suffix_windows: String,
    #[config(default = [])]
    pub(crate) omw_cs_data_paths_list: Vec<String>,
    /// [Section: Turn normal grass]
    #[config(default = ":")]
    pub(crate) turn_normal_grass_stat_ids_separator: String,
    #[config(default = 100_u8)]
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
    #[config(default = 1.3_f32)]
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
    /// [Section: Backup file suffixes]
    #[config(default = ".backup")]
    pub(crate) settings_backup_suffix: String,
    #[config(default = ".backup")]
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
    #[config(default = "Stats:")]
    pub(crate) prefix_list_stats: String,
    #[config(default = "Ignored important error: ")]
    pub(crate) prefix_ignored_important_error_message: String,
    #[config(
        default = "\n\tConsider reporting the error to add this tag to the list of unexpected tags to skip by default"
    )]
    pub(crate) infix_add_unexpected_tag_suggestion: String,
    #[config(
        default = "\n\tFix the problem or add \"--ignore-important-errors\"(may rarely cause unexpected behaviour) to ignore"
    )]
    pub(crate) suffix_add_ignore_important_errors_suggestion: String,
}
