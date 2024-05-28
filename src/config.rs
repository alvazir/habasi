use crate::{create_dir_early, ListOptions};
use anyhow::{Context, Result};
use fs_err::write;
use hashbrown::{HashMap, HashSet};
use std::{ffi::OsString, path::PathBuf};
mod options;
mod settings;
mod util;
use confique::toml::{template, FormatOptions};
use options::{get_options, Options};
use settings::{get_settings, Settings};
use util::{
    backup_settings_file, check_base_dir, check_mode, check_settings_version, get_exe_name_and_dir,
    get_lists, get_log_file, get_settings_file, make_keep_only_last_info_ids, make_tng_stat_ids,
    prepare_plugin_extensions_to_ignore, set_low_string_osstring, set_new_name_retries,
};

#[allow(clippy::struct_excessive_bools)]
pub struct Cfg {
    pub(crate) merge: Vec<Vec<String>>,
    pub(crate) log: Option<PathBuf>,
    pub(crate) no_log: bool,
    pub(crate) settings_file: SettingsFile,
    pub(crate) grass: bool,
    pub(crate) verbose: u8,
    pub(crate) quiet: bool,
    pub(crate) show_plugins: bool,
    pub(crate) list_options: ListOptions,
    pub(crate) presets: Presets,
    pub(crate) advanced: Advanced,
    pub(crate) guts: Guts,
}

pub struct Advanced {
    pub(crate) grass_filter: Vec<String>,
    pub(crate) turn_normal_grass_stat_ids: TngStatIds,
    pub(crate) keep_only_last_info_ids: HashMap<String, HashMap<String, String>>,
}

pub struct Guts {
    // [Section: Presets]
    pub(crate) preset_config_turn_normal_grass: Vec<String>,
    pub(crate) preset_config_turn_normal_grass_add_with_check_references: Vec<String>,
    pub(crate) preset_config_check_references: Vec<String>,
    pub(crate) preset_config_merge_load_order: Vec<String>,
    pub(crate) preset_config_merge_load_order_grass: Vec<String>,
    pub(crate) preset_config_merge_load_order_add_with_check_references: Vec<String>,
    pub(crate) preset_config_merge_load_order_add_with_turn_normal_grass: Vec<String>,
    // [Section: Game configuration file]
    pub(crate) config_path_suffix_linux_macos: String,
    pub(crate) config_path_suffix_windows: String,
    pub(crate) config_paths_list: Vec<String>,
    pub(crate) mor_line_beginning_content: String,
    pub(crate) mor_line_beginning_archive: String,
    pub(crate) mor_line_missing_archive: String,
    pub(crate) mor_data_files_dir: String,
    pub(crate) omw_line_beginning_content: String,
    pub(crate) omw_line_beginning_data: String,
    pub(crate) omw_line_beginning_fallback_archive: String,
    pub(crate) omw_line_beginning_groundcover: String,
    pub(crate) omw_plugin_extensions: Vec<OsString>,
    pub(crate) plugin_extensions_to_ignore: Vec<String>,
    pub(crate) unexpected_tags_to_ignore: Vec<String>,
    pub(crate) skipped_processing_plugins_msg_verbosity: u8,
    // [Section: "Hidden" OpenMW-CS data directory]
    pub(crate) omw_cs_data_path_suffix_linux_macos: String,
    pub(crate) omw_cs_data_path_suffix_windows: String,
    pub(crate) omw_cs_data_paths_list: Vec<String>,
    // [Section: Turn normal grass]
    pub(crate) turn_normal_grass_new_name_retries: u8,
    pub(crate) turn_normal_grass_plugin_name_suffix_deleted_content: String,
    pub(crate) turn_normal_grass_plugin_name_suffix_grass: String,
    pub(crate) turn_normal_grass_header_author_append: String,
    pub(crate) turn_normal_grass_header_description_content: String,
    pub(crate) turn_normal_grass_header_description_groundcover: String,
    // [Section: Meshes]
    pub(crate) mesh_extension: StringOsPath,
    pub(crate) meshes_dir: StringOsPath,
    pub(crate) grass_subdir: StringOsPath,
    // [Section: Header]
    pub(crate) header_version: f32,
    pub(crate) header_author: String,
    pub(crate) header_description_merged_many_plugins_prefix: String,
    pub(crate) header_description_merged_many_plugins_suffix: String,
    pub(crate) header_description_processed_one_plugin_prefix: String,
    pub(crate) header_description_processed_one_plugin_suffix: String,
    // [Section: Backup files suffixes]
    pub(crate) log_backup_suffix: String,
    // [Section: Prefixes of per list options that take paths]
    pub(crate) list_options_prefix_base_dir: String,
    pub(crate) list_options_prefix_config: String,
    pub(crate) list_options_prefix_append_to_use_load_order: String,
    pub(crate) list_options_prefix_skip_from_use_load_order: String,
    // [Section: Messages]
    pub(crate) prefix_combined_stats: String,
    pub(crate) prefix_list_stats: String,
    pub(crate) prefix_ignored_important_error_message: String,
    pub(crate) infix_add_unexpected_tag_suggestion: String,
    pub(crate) suffix_add_ignore_important_errors_suggestion: String,
}

pub struct SettingsFile {
    pub(crate) path: PathBuf,
    pub(crate) version_message: Option<String>,
    pub(crate) write: bool,
    pub(crate) backup_path: PathBuf,
    pub(crate) backup_written: bool,
    pub(crate) backup_overwritten: bool,
}

#[allow(clippy::struct_excessive_bools)]
pub struct Presets {
    pub(crate) present: bool,
    pub(crate) check_references: bool,
    pub(crate) merge_load_order: bool,
    pub(crate) turn_normal_grass: bool,
}

pub struct StringOsPath {
    pub(crate) string: String,
    pub(crate) os_string: OsString,
    pub(crate) path_buf: PathBuf,
}

pub struct TngStatIds {
    pub(crate) set: HashSet<String>,
    pub(crate) source_map: HashMap<String, String>,
}

impl Cfg {
    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
    fn new(
        opt: Options,
        set: Settings,
        settings_file: SettingsFile,
        exe: Option<String>,
        dir: Option<PathBuf>,
    ) -> Result<Self> {
        macro_rules! opt_or_set_bool {
            ($name:ident) => {
                match opt.$name {
                    true => opt.$name,
                    false => set.options.$name,
                }
            };
        }
        macro_rules! opt_or_set_some {
            ($name:ident) => {
                match opt.$name {
                    Some(value) => value,
                    None => set.options.$name,
                }
            };
        }
        macro_rules! set_ext_vec {
            ($name:expr) => {
                $name.iter().map(|ext| ext.to_lowercase().into()).collect()
            };
        }
        let no_log = opt_or_set_bool!(no_log);
        let mode = opt_or_set_some!(mode);
        let base_dir_string = opt_or_set_some!(base_dir);
        let preset_check_references = opt_or_set_bool!(preset_check_references);
        let preset_merge_load_order = opt_or_set_bool!(preset_merge_load_order);
        let preset_turn_normal_grass = opt_or_set_bool!(preset_turn_normal_grass);
        Ok(Self {
            merge: get_lists(opt.merge, set.options.merge, opt.arguments_tail)
                .with_context(|| "Failed to parse --merge lists")?,
            grass: opt_or_set_bool!(grass),
            no_log,
            log: get_log_file(no_log, &opt_or_set_some!(log), exe, dir)?,
            settings_file,
            list_options: ListOptions {
                no_compare: opt_or_set_bool!(no_compare),
                no_compare_secondary: opt_or_set_bool!(no_compare_secondary),
                dry_run: opt_or_set_bool!(dry_run),
                dry_run_secondary: opt_or_set_bool!(dry_run_secondary),
                mode: check_mode(&mode)?,
                base_dir: check_base_dir(&base_dir_string)
                    .with_context(|| "Failed to get default base_dir")?,
                no_ignore_errors: opt_or_set_bool!(no_ignore_errors),
                strip_masters: opt_or_set_bool!(strip_masters),
                reindex: opt_or_set_bool!(reindex),
                debug: opt_or_set_bool!(debug),
                show_all_missing_refs: opt_or_set_bool!(show_all_missing_refs),
                no_show_missing_refs: opt_or_set_bool!(no_show_missing_refs),
                ignore_important_errors: opt_or_set_bool!(ignore_important_errors),
                config: opt_or_set_some!(config),
                exclude_deleted_records: opt_or_set_bool!(exclude_deleted_records),
                use_load_order: opt_or_set_bool!(use_load_order),
                turn_normal_grass: opt_or_set_bool!(turn_normal_grass),
                prefer_loose_over_bsa: opt_or_set_bool!(prefer_loose_over_bsa),
                regex_case_sensitive: opt_or_set_bool!(regex_case_sensitive),
                regex_sort_by_name: opt_or_set_bool!(regex_sort_by_name),
                insufficient_merge: opt_or_set_bool!(insufficient_merge),
                dry_run_dismiss_stats: opt_or_set_bool!(dry_run_dismiss_stats),
                append_to_use_load_order: opt_or_set_some!(append_to_use_load_order),
                skip_from_use_load_order: opt_or_set_some!(skip_from_use_load_order),
            },
            verbose: if opt.verbose == 0 {
                set.options.verbose
            } else {
                opt.verbose
            },
            quiet: opt_or_set_bool!(quiet),
            show_plugins: opt_or_set_bool!(show_plugins),
            presets: Presets {
                present: preset_check_references
                    || preset_merge_load_order
                    || preset_turn_normal_grass,
                check_references: preset_check_references,
                merge_load_order: preset_merge_load_order,
                turn_normal_grass: preset_turn_normal_grass,
            },
            advanced: Advanced {
                grass_filter: set.advanced.grass_filter,
                turn_normal_grass_stat_ids: make_tng_stat_ids(
                    set.advanced.turn_normal_grass_stat_ids,
                    &set.guts.turn_normal_grass_stat_ids_separator,
                )?,
                keep_only_last_info_ids: make_keep_only_last_info_ids(
                    set.advanced.keep_only_last_info_ids,
                )?,
            },
            guts: Guts {
                // [Section: Presets]
                preset_config_turn_normal_grass: set.guts.preset_config_turn_normal_grass,
                preset_config_check_references: set.guts.preset_config_check_references,
                preset_config_merge_load_order: set.guts.preset_config_merge_load_order,
                preset_config_turn_normal_grass_add_with_check_references: set
                    .guts
                    .preset_config_turn_normal_grass_add_with_check_references,
                preset_config_merge_load_order_grass: set.guts.preset_config_merge_load_order_grass,
                preset_config_merge_load_order_add_with_check_references: set
                    .guts
                    .preset_config_merge_load_order_add_with_check_references,
                preset_config_merge_load_order_add_with_turn_normal_grass: set
                    .guts
                    .preset_config_merge_load_order_add_with_turn_normal_grass,
                // [Section: Game configuration file]
                config_path_suffix_linux_macos: set.guts.config_path_suffix_linux_macos,
                config_path_suffix_windows: set.guts.config_path_suffix_windows,
                config_paths_list: set.guts.config_paths_list,
                mor_line_beginning_content: set.guts.mor_line_beginning_content,
                mor_line_beginning_archive: set.guts.mor_line_beginning_archive,
                mor_line_missing_archive: set.guts.mor_line_missing_archive,
                mor_data_files_dir: set.guts.mor_data_files_dir,
                omw_line_beginning_content: set.guts.omw_line_beginning_content,
                omw_line_beginning_data: set.guts.omw_line_beginning_data,
                omw_line_beginning_fallback_archive: set.guts.omw_line_beginning_fallback_archive,
                omw_line_beginning_groundcover: set.guts.omw_line_beginning_groundcover,
                omw_plugin_extensions: set_ext_vec!(set.guts.omw_plugin_extensions),
                plugin_extensions_to_ignore: prepare_plugin_extensions_to_ignore(
                    &set.guts.plugin_extensions_to_ignore,
                ),
                unexpected_tags_to_ignore: set
                    .guts
                    .unexpected_tags_to_ignore
                    .iter()
                    .map(|tag| tag.to_lowercase())
                    .collect(),
                skipped_processing_plugins_msg_verbosity: set
                    .guts
                    .skipped_processing_plugins_msg_verbosity,
                // [Section: "Hidden" OpenMW-CS data directory]
                omw_cs_data_path_suffix_linux_macos: set.guts.omw_cs_data_path_suffix_linux_macos,
                omw_cs_data_path_suffix_windows: set.guts.omw_cs_data_path_suffix_windows,
                omw_cs_data_paths_list: set.guts.omw_cs_data_paths_list,
                // [Section: Turn normal grass]
                turn_normal_grass_new_name_retries: set_new_name_retries(
                    set.guts.turn_normal_grass_mesh_new_name_retries,
                )?,
                turn_normal_grass_plugin_name_suffix_deleted_content: set
                    .guts
                    .turn_normal_grass_plugin_name_suffix_deleted_content,
                turn_normal_grass_plugin_name_suffix_grass: set
                    .guts
                    .turn_normal_grass_plugin_name_suffix_grass,
                turn_normal_grass_header_author_append: set
                    .guts
                    .turn_normal_grass_header_author_append,
                turn_normal_grass_header_description_content: set
                    .guts
                    .turn_normal_grass_header_description_content,
                turn_normal_grass_header_description_groundcover: set
                    .guts
                    .turn_normal_grass_header_description_groundcover,
                // [Section: Meshes]
                mesh_extension: set_low_string_osstring(&set.guts.mesh_extension),
                meshes_dir: set_low_string_osstring(&set.guts.meshes_dir),
                grass_subdir: set_low_string_osstring(&set.guts.grass_subdir),
                // [Section: Header]
                header_version: set.guts.header_version,
                header_author: set.guts.header_author,
                header_description_merged_many_plugins_prefix: set
                    .guts
                    .header_description_merged_many_plugins_prefix,
                header_description_merged_many_plugins_suffix: set
                    .guts
                    .header_description_merged_many_plugins_suffix,
                header_description_processed_one_plugin_prefix: set
                    .guts
                    .header_description_processed_one_plugin_prefix,
                header_description_processed_one_plugin_suffix: set
                    .guts
                    .header_description_processed_one_plugin_suffix,
                // [Section: Backup files suffixes]
                log_backup_suffix: set.guts.log_backup_suffix,
                // [Section: Prefixes of per list options that take paths]
                list_options_prefix_base_dir: set.guts.list_options_prefix_base_dir,
                list_options_prefix_config: set.guts.list_options_prefix_config,
                list_options_prefix_append_to_use_load_order: set
                    .guts
                    .list_options_prefix_append_to_use_load_order,
                list_options_prefix_skip_from_use_load_order: set
                    .guts
                    .list_options_prefix_skip_from_use_load_order,
                // [Section: Messages]
                prefix_combined_stats: set.guts.prefix_combined_stats,
                prefix_list_stats: set.guts.prefix_list_stats,
                prefix_ignored_important_error_message: set
                    .guts
                    .prefix_ignored_important_error_message,
                infix_add_unexpected_tag_suggestion: set.guts.infix_add_unexpected_tag_suggestion,
                suffix_add_ignore_important_errors_suggestion: set
                    .guts
                    .suffix_add_ignore_important_errors_suggestion,
            },
        })
    }
}

pub fn get() -> Result<Cfg> {
    let options = get_options()?;
    let (exe, dir) = get_exe_name_and_dir();
    let mut settings_file = get_settings_file(&exe, &dir, &options)
        .with_context(|| "Failed to get program settings file path")?;
    let settings = get_settings(&mut settings_file)
        .with_context(|| "Failed to get default or provided settings")?;
    if options.settings_write {
        let toml = template::<Settings>(FormatOptions::default());
        create_dir_early(&settings_file.path, "Settings")?;
        backup_settings_file(&mut settings_file, &settings.guts.settings_backup_suffix)?;
        write(&settings_file.path, toml).with_context(|| {
            format!(
                "Failed to write default program settings into \"{}\"",
                settings_file.path.display()
            )
        })?;
    }
    let configuration = Cfg::new(options, settings, settings_file, exe, dir)
        .with_context(|| "Failed to configure program")?;
    Ok(configuration)
}
