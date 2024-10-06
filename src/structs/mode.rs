use std::fmt;

#[derive(Clone, Default)]
pub enum Mode {
    #[default]
    Keep,
    KeepWithoutLands,
    Jobasha,
    JobashaWithoutLands,
    Grass,
    Replace,
    CompleteReplace,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Self::Keep => "keep",
                Self::KeepWithoutLands => "keep_without_lands",
                Self::Jobasha => "jobasha",
                Self::JobashaWithoutLands => "jobasha_without_lands",
                Self::Grass => "grass",
                Self::Replace => "replace",
                Self::CompleteReplace => "complete_replace",
            }
        )?;
        write!(f, "")
    }
}
