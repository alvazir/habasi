use confique::Config;

#[allow(
    clippy::struct_excessive_bools,
    clippy::doc_markdown,
    clippy::doc_link_with_quotes
)]
#[derive(Config)]
pub struct Options {
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
    ///   - [force_base_dir] "no_force_base_dir", "force_base_dir"
    ///   - [exclude_deleted_records] "no_exclude_deleted_records", "exclude_deleted_records"
    ///   - [no_show_missing_refs] "show_missing_refs", "no_show_missing_refs"
    ///   - [debug] "no_debug", "debug"
    ///   - [no_ignore_errors] "ignore_errors", "no_ignore_errors"
    ///   - [no_compare] "compare", "no_compare"
    ///   - [no_compare_secondary] "compare_secondary", "no_compare_secondary"
    ///   - [dry_run_secondary] "no_dry_run_secondary", "dry_run_secondary"
    ///   - [dry_run_dismiss_stats] "no_dry_run_dismiss_stats", "dry_run_dismiss_stats"
    ///   - [regex_case_sensitive] "no_regex_case_sensitive", "regex_case_sensitive"
    ///   - [regex_sort_by_name] "no_regex_sort_by_name", "regex_sort_by_name"
    ///   - [force_dial_type] "no_force_dial_type", "force_dial_type"
    ///   - [ignore_important_errors] "no_ignore_important_errors", "ignore_important_errors"
    ///   - [insufficient_merge] "no_insufficient_merge", "insufficient_merge"
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
    #[config(default = 0_u8)]
    pub(crate) verbose: u8,
    #[config(default = false)]
    pub(crate) quiet: bool,
    #[config(default = false)]
    pub(crate) show_plugins: bool,
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
    pub(crate) force_base_dir: bool,
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
    pub(crate) regex_case_sensitive: bool,
    #[config(default = false)]
    pub(crate) regex_sort_by_name: bool,
    #[config(default = false)]
    pub(crate) force_dial_type: bool,
    #[config(default = false)]
    pub(crate) ignore_important_errors: bool,
    #[config(default = false)]
    pub(crate) insufficient_merge: bool,
    #[config(default = "")]
    pub(crate) append_to_use_load_order: String,
    #[config(default = "")]
    pub(crate) skip_from_use_load_order: String,
}
