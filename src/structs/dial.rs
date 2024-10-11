use super::{GlobalRecordId, InfoId, InfoName};
use hashbrown::HashMap;
use tes3::esp::{Dialogue, DialogueInfo, DialogueType, DialogueType2};

#[allow(clippy::module_name_repetitions)]
pub struct DialMeta {
    pub(crate) global_dial_id: GlobalRecordId,
    pub(crate) info_metas: HashMap<InfoName, InfoId>,
}

pub struct Dial {
    pub(crate) dialogue: Dialogue,
    pub(crate) info: Vec<DialogueInfo>,
    pub(crate) excluded_infos: Vec<usize>,
    pub(crate) dialogue_type: DialDialogueType,
}

impl Dial {
    pub(crate) const fn new(dial: Dialogue) -> Self {
        Self {
            dialogue_type: DialDialogueType::new(dial.dialogue_type),
            dialogue: dial,
            info: Vec::new(),
            excluded_infos: Vec::new(),
        }
    }
}

#[allow(clippy::module_name_repetitions)]
pub struct DialDialogueType {
    pub(crate) info: DialogueType,
    pub(crate) changed: bool,
}

impl DialDialogueType {
    const fn new(dialogue_type_2: DialogueType2) -> Self {
        Self {
            info: dial_to_info_dialogue_type(dialogue_type_2),
            changed: false,
        }
    }

    pub(crate) fn change(&mut self, dialogue_type_2: DialogueType2) {
        self.info = dial_to_info_dialogue_type(dialogue_type_2);
        self.changed = true;
    }
}

const fn dial_to_info_dialogue_type(dial: DialogueType2) -> DialogueType {
    macro_rules! dial_to_info_dialogue_type {
            ($($variant:ident),+) => {
                match dial {
                    $(DialogueType2::$variant => DialogueType::$variant,)+
                }
            }
        }
    dial_to_info_dialogue_type!(Topic, Voice, Greeting, Persuasion, Journal)
}
