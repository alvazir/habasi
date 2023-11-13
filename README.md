<!-- markdownlint-disable MD013 -->
<!-- markdownlint-disable MD033 -->

# Habasi

TES3 Plugin Merging Tool.  

## Description

Habasi will steal your precious plugins and stash them. It is a [command line](https://en.wikipedia.org/wiki/Command-line_interface) tool for TES3 plugin merging, e.g. it takes multiple plugins and creates one with their contents.  

## Features

* Merge plugins
* Keep mergeable records for record mergers(optional, enabled by default)
* Show detailed information
* Presets to quickly perform additional tasks
  * `-T`: "Turn Normal Grass" turns normal grass and kelp into groundcover(as in original [Hemaris](https://www.nexusmods.com/morrowind/users/102938538)' mods: [1](https://www.nexusmods.com/morrowind/mods/52010), [2](https://www.nexusmods.com/morrowind/mods/52058)) for your setup([Groundcoverify](https://gitlab.com/bmwinger/groundcoverify) is an alternative utility for this task)
  * `-C`: "Check References" reports broken references(outdated plugins)
  * `-O`: "Merge Load Order" merges your whole load order including groundcover

## Usage

* Type command `habasi -h` for brief help
* Type command `habasi --help` for extended help
* Type command `habasi --help-option <OPTION>` to get extended help for a specific option
* Example outputs:  
  <details>
  
  <summary>Brief help</summary>

  ```text
  Habasi - TES3 Plugin Merging Tool
  
  Usage: habasi [OPTIONS]
  
  Options:
    -m, --merge <OUTPUT[, OPTIONS], LIST>...  List(s) of plugins to merge
    -l, --log <PATH>                          Name of the log file
    -L, --no-log                              Do not write log
    -s, --settings <PATH>                     Name of the program settings file
        --settings-write                      Write default program settings file and exit
    -g, --grass                               Process grass lists(enabled by default)
    -?, --help-option <OPTION>                Print help for specific option
    -h, --help                                Print help (see more with '--help')
    -V, --version                             Print version
  
  Presets:
    -C, --preset-check-references   Check for missing references in the whole load order [aliases: check]
    -T, --preset-turn-normal-grass  Turn Normal Grass and Kelp into Groundcover for the whole load order
    -O, --preset-merge-load-order   Merge the whole load order
  
  List options:
    -M, --mode <MODE>                      How to process possibly mergeable records
    -b, --base-dir <PATH>                  Base directory for plugin lists
    -d, --dry-run                          Do not write output plugin
    -u, --use-load-order                   Use plugins list from game config file
    -c, --config <PATH>                    Path to the game config file
    -a, --show-all-missing-refs            Show all missing references
    -t, --turn-normal-grass                Turn Normal Grass and Kelp into Groundcover
    -p, --prefer-loose-over-bsa            Get mesh from BSA only when loose mesh not available
    -r, --reindex                          Reindex references twice
    -S, --strip-masters                    Strip masters when possible
    -E, --exclude-deleted-records          Exclude deleted records with --use-load-order
    -A, --no-show-missing-refs             Do not show missing references
    -D, --debug                            Enable additional debug mode
    -I, --no-ignore-errors                 Do not ignore non-important errors
    -P, --no-compare                       Do not compare output plugin with previous version
        --no-compare-secondary             Do not compare output secondary plugin with previous version
        --dry-run-secondary                Do not write secondary output plugin
        --dry-run-dismiss-stats            Dismiss stats with --dry-run
        --ignore-important-errors          Ignore non-critical errors
        --insufficient-merge               Process only cell references(and statics with '-M grass' or '-t')
        --append-to-use-load-order <PATH>  Append plugin path to --use-load-order list
        --skip-from-use-load-order <NAME>  Skip plugin name from --use-load-order list
  
  Display output:
    -v, --verbose...  Show more information
    -q, --quiet       Do not show anything
    
  ```

  </details>
  <details>
  
  <summary>Program run display output</summary>

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

## Changelog

Please see the [CHANGELOG](CHANGELOG.md) for a release history.

## Releases

[Binary downloads](https://www.nexusmods.com/morrowind/mods/53002) are available for GNU/Linux(x86-64), Android(AArch64), Windows(x86-64(MSVC, GNU)), macOS(x86-64, AArch64).

## Building

<details>

<summary>Habasi is written in Rust, so you'll need to grab a [Rust installation](https://www.rust-lang.org) in order to compile it. Habasi compiles with Rust 1.73.0(stable) or newer</summary>

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
