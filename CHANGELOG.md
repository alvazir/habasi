<!-- markdownlint-disable MD013 -->
<!-- markdownlint-disable MD033 -->
<!-- markdownlint-disable MD036 -->
# Changelog

## 0.2.12 (2023-11-26)

Fixes

* Skip list if all plugins were skipped

## 0.2.11 (2023-11-20)

Fixes

* Ignore plugins that contain non-TES3 record types(LUAL) automatically thanks to GeneralUlfric's report.

## 0.2.10 (2023-11-19)

Miscellaneous

* Change license from dual MIT and UNLICENSE to GNU GPLv3.
* Rename program from "Habasi - TES3 plugin merging tool" to "Habasi - TES3 plugin merging and utility tool".
* Improve filesystem-related error messages.
* Add system requirements to description.
* Fix typos in several places.

## 0.2.9 (2023-11-13)

Fixes

* Remove error when encountering omwscripts plugin, auto-skip processing of omwscripts plugins(or any other type of plugins via setting guts.plugin_extensions_to_ignore).

Miscellaneous

* Slightly improve default display output by moving "Skipped plugin processing ..." messages to verbose mode.
* Update versions of rust and all dependencies, notably tes3 library to latest commit(2fae07a0).

## 0.2.8 (2023-08-21)

Feature enhancements

* Output plugin with only size of master(s) changed is now considered equal to previous version.

Fixes

* Assign ID to SSCR records with empty IDs. New ID is a CRC64 of script name. This solves very rare problem when using multiple plugins(created with OpenMW-CS) with empty ID SSCR in Morrowind.exe. Check log for new IDs or run with -vv.
* Assign ID to SNDG records with empty IDs. New ID is a creature name and sound type data id padded with three zeros, e.g. alit0006 for alit scream. This solves very rare problem when several SNDG records with emptry IDs overwrite each other even if they are for different creatures. Check log for new IDs or run with -vv.

## 0.2.5 (2023-08-16)

Bug fixes

* Move Journal records in front of all the other dialogue types. Morrowind.exe and TES-CS drop journal conditions from dialogues(INFO records) if corresponding Journal records are defined *after*. Thanks to **AstralJam8** for finding the issue and thorough investigating!

Feature enhancements

* Remove XSCL(scale) subrecord from deleted instances. Most files produced are now slightly slimer, which in turn leads to a bit faster loading. Turn Normal Grass -CONTENT plugins get considerable ~20% decrease in size.
* Remove deleted non-external instances from merged plugin.

Fixes

* Introduce keep_only_last_info_ids mechanic(configurable in settings) to exclude 1 problematic INFO record when merging plugis *LGNPC_GnaarMok* and *LGNPC_SecretMasters*(details in KNOWN_ISSUES.md).
* Remove AMBI, WHGT from deleted cells. This fixes OpenMW-CS(and probably OpenMW) error "Loading failed: ESM Error: Previous record contains unread bytes" on loading a cell with both deleted flag(0x0020) and DELE subrecord.

## 0.2.0 (2023-08-13)

**Breaking changes**

* Several option names has been changed.
* References sorting is now better. Merged plugins' contents is almost identical to TES-CS produced plugins. This means that recreating previously made merged plugins may require new game. Habasi would warn you if that's the case.
* Habasi is incompatible with OpenMW 0.47 and earlier starting from this version.
  * OpenMW 0.48 is now stable and contains fix to the long standing bug [[#6067] Moved instances can now be loaded from any point in the instances list of a cell record](https://gitlab.com/OpenMW/openmw/-/issues/6067). I've [fought](https://github.com/Greatness7/tes3/pull/2) the bug previously, but it's not needed anymore.
  * I've uploaded "legacy" 0.2.0 version to [releases page](https://www.nexusmods.com/morrowind/mods/53002) just in case. It contains changes(described further) to make it work with legacy OpenMW versions. Habasi 0.1.0 is also fully functional.
  <details>
  
  <summary>Make following preparations if you want to compile the "legacy" version yourself:</summary>
  
  * Download(1) or fork(2) [tes3](https://github.com/Greatness7/tes3) library.
  * Insert following line into tes3's "libs/esp/src/types/cell.rs" line 241(as of commit 6b6a0ffc):  
    `reference.moved_cell.is_none(), // openmw 0.47 bug that requires MVRF records to be on top`  
    The result should look like that:  
  
    ```rust
            references.sort_by_key(|((mast_index, refr_index), reference)| {
            (
                reference.moved_cell.is_none(), // openmw 0.47 bug that requires MVRF records to be on top
                !reference.persistent(),
    ```
  
  * Point Cargo.toml to the edited tes3 library(change [dependencies.tes3] block):
    * (1)
  
      ```toml
      [dependencies.tes3]
      path = "../tes3" # change to downloaded library path
      default-features = false
      features = ["esp"]
      ```

    * (2)
  
      ```toml
      [dependencies.tes3]
      git = "https://github.com/Greatness7/tes3" # change to your fork's url
      branch = "dev" # change to your branch
      # rev = "6b6a0ffc" # or comment previous, uncomment this one and change to your commit
      default-features = false
      features = ["esp"]
      ```

  * Insert following line into Habasi's "src/util.rs" line 199(as of version 0.2.0):  
    `r.moved_cell.is_none(), // openmw 0.47 bug that requires MVRF records to be on top`  
    The result should look like that:  
  
    ```rust
            references.sort_by_key(|r| {
            (
                r.moved_cell.is_none(), // openmw 0.47 bug that requires MVRF records to be on top
                !r.persistent(),
    ```
  
  </details>

New features

* Presets:
  * `-T`: "Turn Normal Grass" turns normal grass and kelp into groundcover(as in original [Hemaris](https://www.nexusmods.com/morrowind/users/102938538)' mods: [1](https://www.nexusmods.com/morrowind/mods/52010), [2](https://www.nexusmods.com/morrowind/mods/52058)) for your setup.
  * `-C`: "Check References" reports broken references, which usually highlight outdated plugins.
  * `-O`: "Merge Load Order" merges your whole load order including groundcover. This preset may be used for many purposes, for example:
    * Pass merged plugins to [The LawnMower for Morrowind](https://www.nexusmods.com/morrowind/mods/53034) to quickly remove clipping grass for all your plugins.
    * Pass merged plugins to [Waresifier](https://www.nexusmods.com/morrowind/mods/51390) to quickly generate Wares containers for all your plugins.
    * Use instead of your load order for faster loading on potato phone :-) Also it's no longer needed to delete one record from the output plugin when TR_Mainland and TR_Hotfix are among plugins merged. All DELETED records are auto-removed from the result with this preset.
  * Note: Presets utilize newly added option `--use-load-order` which imposes specific requirement(explained further).
  * Note: Presets may be combined. For example `habasi -CTO` performs all three available presets at once and produces slimer results.
* Options:
  * `--use-load-order`: Uses your game configuration file and also reports some configuration errors, e.g. missing plugins. Requires either "old style" Morrowind modding way(dump everything into Data Files) or openmw.cfg. This restriction is due to Morrowind.ini's lack of paths to different mod directories. Morrowind's Mod Organizer users may produce openmw.cfg with [ModOrganizer-to-OpenMW](https://www.nexusmods.com/morrowind/mods/45642), then point Habasi to it with `--config` option.
  * `--config`: Provide path to alternative game configuration file or main configuration file in case auto-detection fails.
  * `--ignore-important-errors`: Ignore missing or corrupted plugins. This option has been requested by [sunhawken](https://forum.nexusmods.com/index.php?showtopic=12928251/#entry125013045).
  * `-?`: Get help for individual option, because extended help `habasi --help` became too long to quickly find something.
  * Few more minor options.

Performance improvements

* Comparison of newly created and previous version of merged plugin is now slighly faster.
* Grass processing also became slightly faster.

Feature enhancements

* Grass processing produces slimer output. Non-grass statics, empty and interior cells are automatically excluded.
* Slightly improved output plugin description with the number of plugins merged.
* There is now message telling first difference to previous version of output plugin.
* Display and log output became slightly better.
* Auto-backup of previous log and settings files.
* Debug mode no longer skips duplicate records.
* Add references count to stats to provide visual explanation why some plugin lists take more time to be processed.
* All list options are now available as global and per list options, allowing more flexibility by combining them.
* More forgiving argument names processing. For example `--dry_run` would be treated as correct form of `--dry-run`, per list option `DRY-RUN` would also be treated as the correct form of `dry_run`.

Fixes

* Reword `--mode` help section slightly, add note about [DeltaPlugin](https://gitlab.com/bmwinger/delta-plugin). I made a [mistake](https://github.com/alvazir/habasi/issues/1) when [wrote](https://www.reddit.com/r/tes3mods/comments/13xnji3/habasi_tes3_plugin_merging_tool/) that DeltaPlugin would work with `keep` or `replace` modes the same way as if it worked with unmerged plugins. DeltaPlugin processes records the same way as both engines, e.g. discards different variants of mergeable records except the last one. Possible way to use both utilities is to make additional openmw.cfg file with paths to unmerged plugins, then run `delta_plugin -c openmw.cfg`, then run `habasi`.

Miscellaneous

* Almost half of the initial code has been edited, the amount of code added is roughly equal to initial. That means bugs probability has been increased :-)

## 0.1.0 (2023-06-01)

Features

* Merge plugins
* Keep mergeable records for record mergers(optional, enabled by default)
* Show detailed information
