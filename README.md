<!-- markdownlint-disable MD013 -->
<!-- markdownlint-disable MD033 -->

# Habasi

TES3 Plugin Merging Tool.  

## Description

Habasi will steal your precious plugins and stash them. It is a [command line](https://en.wikipedia.org/wiki/Command-line_interface) tool for TES3 plugin merging, e.g. it takes multiple plugins and creates one with their contents.  

Features:  
[x] Merge plugins  
[x] Keep mergeable records for record mergers(optional, enabled by default)  
[x] Show detailed information  
[ ] Scan load order for easier "missing references" detection  
[ ] (maybe) Automatic creation of "turn normal grass and kelp into groundcover" and similar tasks for your specific setup  

## Usage

<details>

<summary>Type command `habasi -h` for brief help</summary>

```text
Habasi - TES3 Plugin Merging Tool

Usage: habasi [OPTIONS]

Options:
  -m, --merge <OUTPUT[, OPTIONS], LIST>...  List(s) of plugins to merge
  -C, --no-compare                          Do not compare output plugin with previous version
  -g, --grass                               Process grass lists(enabled by default)
  -l, --log <PATH>                          Name of the log file
  -L, --no-log                              Do not write log
  -s, --settings <PATH>                     Name of the program settings file
      --settings-write                      Write default program settings file and exit
  -h, --help                                Print help (see more with '--help')
  -V, --version                             Print version

List options:
  -d, --dry-run                   Do not write output plugin(s)
  -M, --mode <MODE>               How to process possibly mergeable records
  -B, --base-dir <base_dir:PATH>  Base directory for plugin lists
  -I, --no-ignore-errors          Do not ignore non-important errors
  -S, --strip-masters             Strip masters when possible
  -R, --reindex                   Reindex references twice
  -D, --debug                     Debug

Display output:
  -v, --verbose...             Show more information
  -q, --quiet                  Do not show anything
  -a, --show-all-missing-refs  Show all missing references

```

</details>
<details>

<summary>Type command `habasi --help` for extended help</summary>

```text
Habasi - TES3 Plugin Merging Tool

Author: alvazir
License: Unlicense OR MIT
GitHub: https://github.com/alvazir/habasi
Nexus Mods: https://www.nexusmods.com/morrowind/mods/53002

Usage: habasi [OPTIONS]

Options:
  -m, --merge <OUTPUT[, OPTIONS], LIST>...
          List(s) of plugins to merge. This option is handy for one-shot merges. Settings file should be more convenient for "permanent" or
          longer lists , see --settings.

          Each list is a double-quoted(*) string that consists of output plugin name, optional list options("replace" in second example) and
          comma-separated list of plugins to merge. Ouput plugin's name should come first. Examples:
            "MergedGhostRevenge.esp, GhostRevenge.ESP, GhostRevenge_TR1912.esp"
            "MergedPlugin01.esp, replace, Frozen in Time.esp, The Minotaurs Ring.esp, Cure all the Kwama Queens.ESP"

          May be repeated. May take either one or multiple comma-separated lists(no space after comma). Following examples are identical:
            -m "MergedGhostRevenge.esp, GhostRevenge.ESP, GhostRevenge_TR1912.esp" -m "MergedPlugin01.esp, replace, Frozen in Time.esp, The
            Minotaurs Ring.esp, Cure all the Kwama Queens.ESP"
            --merge "MergedGhostRevenge.esp, GhostRevenge.ESP, GhostRevenge_TR1912.esp","MergedPlugin01.esp, replace, Frozen in Time.esp, The
            Minotaurs Ring.esp, Cure all the Kwama Queens.ESP"

          List options may be set globally and per each list. List specific options override global options. See each of the list options
          details in corresponding help sections.

          (*) Windows-style paths with backslash symbol '\' require special care. You may:
            - Replace backslash with slash, e.g. '\' => '/'
            - Prepend backslash with another slash(so-called escaping), e.g. '\' => '\\'
            - Enclose string into single quotes instead of double quotes. If path contains single quote itself, then enclose string into triple
            single quotes
            Examples:
              - "D:/Data Files" = "D:\\Data Files" = 'D:\Data Files' = '''D:\Data Files'''
              - "C:/mods/mod with quote'.esp" = "C:\\mods\\mod with quote'.esp" = '''C:\mods\mod with quote'.esp'''

  -C, --no-compare
          Do not compare output plugin with previous version if it exists.

          By default program doesn't overwrite previous version of output plugin if it's not changed. Disabling comparison would slightly
          improve processing time.

  -g, --grass
          Process grass lists(enabled by default).

          Grass rarely changes and it's processing may take more time then other plugins combined due to the size. Consider setting this option
          to "false" in settings file and then use this flag sometimes.

  -l, --log <PATH>
          Name of the log file. May be provided as a path. Non-existent directories will be created.

          Log contains display output of the program as if it was run with maximum verboseness. It is enabled by default, use --no-log to
          disable.

          Default value: "<program_name>.log"(file will be created in program directory).

  -L, --no-log
          Do not write log

  -s, --settings <PATH>
          Name of the program settings file. May be provided as a path. Non-existent directories will be created. Extension will be replaced
          with ".toml".

          Default value: "<program_name>.toml"(file will be created in program directory).

      --settings-write
          Write default program settings file and exit.

          Use this option if you keep using the same arguments. Modify default settings to suit your needs.

          File will be created in program directory with name "<program_name>.toml" by default. Use --settings to provide another path. Keep in
          mind that non-default settings file path should be explicitly provided every time you want to use it.

          This flag conflicts with everything except --settings, --log, --no-log.

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

List options:
  -d, --dry-run
          Do not write output plugin(s).

          Corresponding per list options: "dry_run", "no_dry_run".

  -M, --mode <MODE>
          Mode defines how to process possibly mergeable record. Available modes are:

            "keep"
              - All possibly mergeable records are stacked in the output plugin, so that record merging utilities(TES3Merge, Delta Plugin,
              Merged Lands etc) would be able to do their work as if plugins were not merged together. Nothing would break if you don't use any
              record merging utilities at all.

            "keep_without_lands"
              - Same as "keep", but LAND records(landscape) would simply be replaced. You may use this mode if you don't intend to use "Merged
              Lands".

            "replace"
              - All records are replaced(hence no records to merge, that's how Morrowind works with records), except leveled lists. Leveled list
              merging utilities(tes3cmd, OMWLLF, Jobasha, TES3Merge, Delta Plugin etc) would be able to do their work as if plugins were not
              merged together. You may use this mode if you don't merge any records except leveled lists. Engine processes merged plugins of
              "keep" and "replace" modes exactly the same, but "replace" produces slightly smaller results.

            "complete_replace"
              - Same as replace, but leveled lists are replaced too. You may use this mode if you don't merge anything. I'd only recommend this
              mode for minimalistic mode setups, but why'd you need plugin merger then?

            "grass"
              - Same as replace, but designed for grass. Allows excluding instances that you don't like. By default it excludes "UNKNOWN GRASS"
              records from Remiros' Groundcover. Check "grass_filter" option in settings file if you want to exclude anything else(i.e.
              mushrooms).

          Default value: "keep".

  -B, --base-dir <base_dir:PATH>
          Base directory for plugin lists.

          By default program uses current directory("base_dir:off") as a base for plugin's relative paths. Plugin's absolute paths obviously
          ignore this option.

          Example:
            -m "BTBGIsation - Custom Merged.esp, base_dir:mods/Patches/BTBGIsation/03 Modular - Secondary, BTBGIsation - Magical Missions.esp,
            BTBGIsation - Weapons Expansion Morrowind.esp"

          Default value: "base_dir:off".

  -I, --no-ignore-errors
          Do not ignore non-important errors.

          By default program ignores external references that are missing in master, mimicing game engines behaviour. Those references are
          simply not placed into the output plugin.

          Corresponding per list options: "no_ignore_errors", "ignore_errors".

  -S, --strip-masters
          Strip masters when possible.

          Master-file subrecords are placed into the output plugin's header. They are not strictly required for some plugins, e.g. grass plugins
          or any other plugin that doesn't have external cell references. Program would strip master subrecords when enabled. It's all or
          nothing operation. One or more of external cell references would result in keeping all the master subrecords.

          Corresponding per list options: "strip_masters", "no_strip_masters".

  -R, --reindex
          Reindex references twice.

          References are numbered as they appear by default. Cells would contain non-continious ranges of reference ids as a result. Use this
          option to reindex references again at the expense of additional processing time(up to 30%). This option doesn't change anything
          gameplay wise, it only makes output plugin internally look like it was produced by TES-CS.

          Corresponding per list options: "reindex", "no_reindex".

  -D, --debug
          All versions of record would be placed into the output plugin. May be useful for investigating record mutations.

          Corresponding per list options: "debug", "no_debug".

Display output:
  -v, --verbose...
          Show more information. May be provided multiple times for extra effect:

            -v: Show list options, total stats per list, "references reindexed" and "master subrecords stripped" messages.

            -vv: Show detailed total stats, ignored reference errors, "processing plugin" messages.

            -vvv: Show detailed list stats.

          This flag conflicts with --quiet.

  -q, --quiet
          Do not show anything.

          This flag conflicts with --verbose.

  -a, --show-all-missing-refs
          Show all missing references.

          By default only first missing reference per cell is logged to prevent noise.

Notes:
  - Display/log output looks better with monospaced font.
  - Don't clean the output plugin. It's not designed to be cleaned.
  - Cell references added by merged plugins(unlike external references coming from masters) are reindexed, so starting new game is required to
  use such merged plugins. Similar message is displayed for every written output plugin that contains internal non-external references.

```

</details>
<details>

<summary>Example display output</summary>

```text
$ ./habasi
Log is being written into "/home/alvazir/__OMW/habasi.log"
Output plugin "mods/Patches/Habasi/0/united/United-GRS.esp" was written
Output plugin "mods/Patches/Habasi/0/united/United-100.esp" was written. It contains reindexed references most likely, so new game is recommended.
Output plugin "mods/Patches/Habasi/0/united/United-200.esp" was written. It contains reindexed references most likely, so new game is recommended.
Output plugin "mods/Patches/Habasi/0/united/United-700.esp" was written. It contains reindexed references most likely, so new game is recommended.
Combined plugin lists stats:
  input(625 plugins): 219277 processed, 5111 removed(dup), 6307 merged, 1718 replaced, 7923 instances filtered(grass)
  output(4 plugins): 208570 total, 206141 unique, 2237 mergeable(unique), 2429 mergeable(total), 5.413s duration

```

</details>

## Releases

[Binary downloads](https://www.nexusmods.com/morrowind/mods/53002) are available for GNU/Linux(x86-64), Android(AArch64), Windows(x86-64(MSVC, GNU)), macOS(x86-64, AArch64).

## Building

<details>

<summary>Habasi is written in Rust, so you'll need to grab a [Rust installation](https://www.rust-lang.org) in order to compile it. Habasi compiles with Rust 1.69.0(stable) or newer</summary>

```shell
git clone https://github.com/alvazir/habasi
cd habasi
cargo build --release
./target/release/habasi --version
```

</details>

## Links

* [Nexus Mods releases](https://www.nexusmods.com/morrowind/mods/53002)  
  * [Report a bug](https://www.nexusmods.com/morrowind/mods/53002/?tab=bugs)  
  * [File a feature request/suggestion](https://www.nexusmods.com/morrowind/mods/53002/?tab=posts)  
* [GitHub repository](https://github.com/alvazir/habasi)  
  * [File an issue](https://github.com/alvazir/habasi/issues)  

## License

[Dual-licensed](COPYING) under the [MIT License](LICENSE-MIT) or the [Unlicense](UNLICENSE).  

## Acknowledgments

* This project came to life thanks to the awesome [tes3 library](https://github.com/Greatness7/tes3) by [Greatness7](https://github.com/Greatness7)  
