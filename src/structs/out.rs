use super::Dial;
use tes3::esp::{
    Activator, Alchemy, Apparatus, Armor, Birthsign, Bodypart, Book, Cell, Class, Clothing,
    Container, Creature, Door, Enchanting, Faction, GameSetting, GlobalVariable, Ingredient,
    Landscape, LandscapeTexture, LeveledCreature, LeveledItem, Light, Lockpick, MagicEffect,
    MiscItem, Npc, PathGrid, Probe, Race, Region, RepairItem, Script, Skill, Sound, SoundGen,
    Spell, StartScript, Static, Weapon,
};

macro_rules! make_out {
    ($($type:ident, $obj:ident);+) => {
        #[derive(Default)]
        pub struct Out {
            pub(crate) masters: Vec<(String, u64)>,
            $(pub(crate) $type: Vec<($obj, Vec<$obj>)>,)+
        }
    };
}

make_out!(gmst, GameSetting; glob, GlobalVariable; clas, Class; fact, Faction; race, Race; soun, Sound; sndg, SoundGen; skil, Skill; mgef, MagicEffect; scpt, Script; regn, Region; bsgn, Birthsign; sscr, StartScript; ltex, LandscapeTexture; spel, Spell; stat, Static; door, Door; misc, MiscItem; weap, Weapon; cont, Container; crea, Creature; body, Bodypart; ligh, Light; ench, Enchanting; npc_, Npc; armo, Armor; clot, Clothing; repa, RepairItem; acti, Activator; appa, Apparatus; lock, Lockpick; prob, Probe; ingr, Ingredient; book, Book; alch, Alchemy; levi, LeveledItem; levc, LeveledCreature; cell, Cell; land, Landscape; pgrd, PathGrid; dial, Dial);
