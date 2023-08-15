<!-- markdownlint-disable MD013 -->
<!-- markdownlint-disable MD033 -->
# Known issues

1. Very rare INFO records with the same ID and DIAL may cause problems  
  **Status**: partially solved  
  **Description**: It's very rare case. I've only seen it when trying to merge `LGNPC_GnaarMok` and `LGNPC_SecretMasters`.  
  Impact is also unknown. I've only experienced it when opening file in OpenMW-CS. Error is `Loading failed: attempt to change the ID of a record`.  
  There is nothing special about those INFO records. The situation is probably atypical though. Both plugins create `threaten` DIAL with 2 INFOs each, first is placeholder, second is the problematic INFO with ID `19511310302976825065`.  
  Solved by creating keep_only_last_info_ids mechanic. It performs following steps:  
  \- takes INFO IDs(and DIAL) from `advanced.settings.keep_only_last_info_ids`  
  \- on each incoming INFO if there is already the one with the same ID and DIAL the older gets excluded from the result  
