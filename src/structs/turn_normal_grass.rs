use super::{FileInBsa, LoadOrder};
use crate::Bsa;
use anyhow::{anyhow, Context as _, Result};
use fs_err::read;
use std::path::PathBuf;
use tes3::esp::Static;

#[derive(Default)]
pub struct TurnNormalGrass {
    pub(crate) stat_records: Vec<Static>,
    pub(crate) loose: Option<PathBuf>,
    pub(crate) bsa: Option<FileInBsa>,
    pub(crate) new_name_low: String,
    pub(crate) new_path: PathBuf,
    pub(crate) file_contents: Vec<u8>,
    pub(crate) src_info: String,
}

impl TurnNormalGrass {
    pub(crate) fn read_from_bsa(&mut self, bsas: &[Bsa]) -> Result<()> {
        self.file_contents = match self.bsa {
            None => {
                return Err(anyhow!(
                    "Bug: trying to read from BSA, though there is no info about BSA"
                ))
            }
            Some(ref bsa) => {
                let bsas_bsa = &bsas.get(bsa.bsa_index).with_context(|| {
                    format!(
                        "Bug: indexing slicing bsas[bsa.bsa_index = {}]",
                        bsa.bsa_index
                    )
                })?;
                self.src_info = format!("mesh \"{}\" from BSA \"{}\"", bsa.path, bsas_bsa.path);
                bsas_bsa
                    .get_file_by_index(bsa.file_index)
                    .with_context(|| {
                        format!(
                            "Failed to get file \"{}\" by index from BSA \"{}\"",
                            bsa.path, bsas_bsa.path
                        )
                    })?
            }
        };
        Ok(())
    }

    pub(crate) fn read_from_loose(&mut self) -> Result<()> {
        self.file_contents = match self.loose {
            None => {
                return Err(anyhow!(
                    "Bug: trying to read from loose file, though there is no info about loose file"
                ))
            }
            Some(ref path) => match read(path) {
                Ok(file) => {
                    self.src_info = format!("loose mesh \"{}\"", path.display());
                    file
                }
                Err(err) => {
                    return Err(anyhow!(
                        "Failed to read from file \"{}\", {}",
                        path.display(),
                        err
                    ))
                }
            },
        };
        Ok(())
    }

    pub(crate) fn should_read_from_loose(&self, load_order: &LoadOrder) -> Result<bool> {
        let loose_time = match self.loose {
            None => {
                return Err(anyhow!(
                "Bug: trying to get time from loose file, though there is no info about loose file"
            ))
            }
            Some(ref loose) => loose.metadata().map_or(None, |meta| meta.modified().ok()),
        };
        let bsa_time = match self.bsa {
            None => return Err(anyhow!("Bug: trying to read time from BSA, though there is no info about BSA")),
            Some(ref bsa) => match load_order.fallback_archives.get(bsa.bsa_index) {
                None => {
                    return Err(anyhow!(
                        "Bug: trying to get time from BSA, though there is no info about BSA with index \"{}\"",
                        bsa.bsa_index
                    ))
                }
                Some(&(_, _, time)) => time,
            },
        };
        let res = loose_time.is_none()
            || bsa_time.is_none()
            || loose_time.with_context(|| "Bug: loose_time is none despite the is_none() check")?
                >= bsa_time.with_context(|| "Bug: bsa_time is none despite the is_none() check")?;
        Ok(res)
    }
}
