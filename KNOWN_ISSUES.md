<!-- markdownlint-disable MD013 -->
<!-- markdownlint-disable MD033 -->
# Known issues

1. Very rare INFO records with the same ID and DIAL may cause problems  
  **Status**: partially solved  
  **Description**: It's a very rare case. I've only seen it when trying to merge `LGNPC_GnaarMok` and `LGNPC_SecretMasters`.  
  Impact is also unknown. I've only experienced it when opening file in OpenMW-CS. Error is `Loading failed: attempt to change the ID of a record`.  
  There is nothing special about those INFO records. The situation is probably atypical though. Both plugins create `threaten` DIAL with 2 INFOs each, first is placeholder, second is the problematic INFO with ID `19511310302976825065`.  
  Solved by creating keep_only_last_info_ids mechanic. It performs following steps:  
  \- takes INFO IDs(and DIAL) from `advanced.settings.keep_only_last_info_ids`  
  \- on each incoming INFO if there is already the one with the same ID and DIAL the older gets excluded from the result  

2. Very rare SSCR records with empty ID may misbehave in Morrowind.exe  
  **Status**: mostly solved  
  **Description**: It's a very rare case. It may cause problems only when using plugins made with OpenMW-CS(containing SSCR) in Morrowind.exe.  
  OpenMW and Morrowind.exe process SSCR records differently. OpenMW doesn't even look at ID, only noticing Script mentioned. OpenMW-CS may create SSCR with empty IDs. That means that multiple plugins with empty id SSCRs would have their SSCRs overwritten.  
  Solved by assigning IDs to SSCRs with empty IDs. New ID is a CRC64 of Script name, so it should also be the same for the same Script name. Check log for new IDs or run with -vv.  

3. Very rare SNDG records with empty ID may be overwritten for different creature  
  **Status**: solved  
  **Description**: It's a very rare case. It doesn't really have any serious consequences.  
  Both engines process SNDG records identically. Several SNDG records with empty IDs would overwrite each other even if they are for different creatures.
  Solved by assigning IDs to SNDGs with empty IDs. New ID is a creature name(truncated to 28 characters), 000 and id of the sound type data id(0-7). E.g. alit scream would be `alit0006`. Check log for new IDs or run with -vv.  

4. Very rare INFO records' type differ from parent DIAL's type  
  **Status**: solved  
  **Description**: OpenMW-CS 0.48(-?) [caused](https://gitlab.com/OpenMW/openmw/-/issues/7861) it, [fixed](https://gitlab.com/OpenMW/openmw/-/merge_requests/3998) in 0.49.  
  [INFO](https://en.m.uesp.net/wiki/Morrowind_Mod:Mod_File_Format/INFO) records contain redundant dialogue type field that should be equal to parent [DIAL](https://en.m.uesp.net/wiki/Morrowind_Mod:Mod_File_Format/DIAL)'s. The program shows warning when it's not.  
  Solved by setting DIAL's type to problematic INFO records with `--force-dial-type` option. Check log for details or run with -v.  
