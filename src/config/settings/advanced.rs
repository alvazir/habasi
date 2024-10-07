use confique::Config;

#[allow(clippy::doc_markdown, clippy::doc_link_with_quotes)]
#[derive(Config)]
pub struct Advanced {
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
