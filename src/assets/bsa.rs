// COMMENT: https://en.uesp.net/wiki/Morrowind_Mod:BSA_File_Format and linked ghostwheel's site
use anyhow::{anyhow, Context, Result};
use fs_err::File;
use std::{
    io::{Read, Seek, SeekFrom},
    str::from_utf8,
};

pub struct Bsa {
    pub(crate) path: String,
    pub(crate) names: Vec<String>,
    data_offset: u64,
    meta: Vec<u8>,
}

impl Bsa {
    pub(crate) fn new(path: &str) -> Result<Self> {
        let bsa = File::open(path).with_context(|| "Failed to open file for reading")?;
        let header = read_n_u8(&bsa, 12).with_context(|| "Failed to read header")?;
        let magic = four_u8_to_le_u32(&header, 0)
            .with_context(|| "Failed to read magic number from header")?;
        // COMMENT: le hex 0x00000100 = 256
        if magic != 256 {
            return Err(anyhow!(
                "Magic number(decimal) is wrong: expected \"256\", got \"{magic}\". Program expects Morrowind's format BSA."
            ));
        }
        let meta_size = four_u8_to_le_u32(&header, 4)
            .with_context(|| "Failed to read meta size from header")?;
        let num_files = four_u8_to_le_u32(&header, 8)
            .with_context(|| "Failed to read files quantity from header")?;
        let meta_size_usize = usize::try_from(meta_size).with_context(|| {
            format!("Bug: failed to cast \"{meta_size}\"(meta_size, u32) to usize")
        })?;
        let num_files_usize = usize::try_from(num_files).with_context(|| {
            format!("Bug: failed to cast \"{num_files}\"(num_files, u32) to usize")
        })?;
        // COMMENT: 12 + meta_size_usize + 8 * num_files_usize
        let data_offset_usize = num_files_usize
            .checked_mul(8_usize)
            .and_then(|v| v.checked_add(meta_size_usize))
            .and_then(|v| v.checked_add(12_usize))
            .with_context(|| {
                format!("Bug: overflow calculating data_offset_usize = 12 + \"{meta_size_usize}\" + 8 * \"{num_files_usize}\"")
            })?;
        let meta = read_n_u8(&bsa, meta_size).with_context(|| "Failed to read meta")?;
        let mut names = Vec::with_capacity(num_files_usize);
        let names_string = from_utf8(
            meta.get(
                num_files_usize.checked_mul(12_usize).with_context(|| {
                    format!(
                        "Bug: overflow multiplying 12 by num_files_usize = \"{num_files_usize}\""
                    )
                })?..,
            )
            .with_context(|| format!("Bug: indexing slicing meta[12 * {num_files_usize}]"))?,
        )
        .with_context(|| {
            "Failed to parse ANSI string with file names. File is probably corrupted."
        })?;
        names.extend(names_string.split('\0').map(ToOwned::to_owned));
        let data_offset = u64::try_from(data_offset_usize).with_context(|| {
            format!("Bug: failed to cast \"{data_offset_usize}\"(data_offset_usize, usize) to u64")
        })?;
        Ok(Self {
            path: path.to_owned(),
            names,
            data_offset,
            meta,
        })
    }

    pub(crate) fn get_file_by_index(&self, file_index: usize) -> Result<Vec<u8>> {
        let index = file_index.checked_mul(8).with_context(|| {
            format!("Bug: overflow multiplying 8 by file_index = \"{file_index}\"")
        })?;
        let file_size = four_u8_to_le_u32(&self.meta, index)
            .with_context(|| "Failed to read file size from meta")?;
        let file_offset = u64::from(
            four_u8_to_le_u32(
                &self.meta,
                index
                    .checked_add(4)
                    .with_context(|| format!("Bug: overflow adding 4 to index = \"{index}\""))?,
            )
            .with_context(|| "Failed to read file offset from meta")?,
        );
        let mut file = File::open(&self.path)
            .with_context(|| format!("Failed to open bsa file \"{}\" for reading", &self.path))?;
        let seek_start = self.data_offset.checked_add(file_offset).with_context(|| {
            format!(
                "Bug: overflow adding file_offset = \"{file_offset}\" to self.data_offset = \"{}\"",
                self.data_offset
            )
        })?;

        file.seek(SeekFrom::Start(seek_start)).with_context(|| {
            format!(
                "Failed to start reading file from bsa file \"{}\" at offset \"{}\"",
                &self.path, seek_start
            )
        })?;
        read_n_u8(&file, file_size).with_context(|| "Failed to read file")
    }
}

fn four_u8_to_le_u32(buffer: &[u8], offset: usize) -> Result<u32> {
    let four_u8: [u8; 4] = buffer
        .get(
            offset
                ..offset
                    .checked_add(4)
                    .with_context(|| format!("Bug: overflow adding 4 to offset = \"{offset}\""))?,
        )
        .with_context(|| format!("Bug: indexing slicing buffer[{offset}..{offset} + 4]"))?
        .try_into()
        .with_context(|| "Failed to get 4 bytes from buffer")?;
    #[allow(clippy::little_endian_bytes)]
    Ok(u32::from_le_bytes(four_u8))
}

fn read_n_u8(file: &File, take: u32) -> Result<Vec<u8>> {
    let mut handle = file.take(u64::from(take));
    let take_usize = usize::try_from(take)
        .with_context(|| format!("Bug: failed to cast \"{take}\"(take, u32) to usize"))?;
    let mut buffer = Vec::with_capacity(take_usize);
    match handle.read_to_end(&mut buffer) {
        Ok(num) => {
            if num == take_usize {
                Ok(buffer)
            } else {
                Err(anyhow!(
                    "Failed to read bytes into buffer: expected {take:?}, got {num:?}"
                ))
            }
        }
        Err(err) => Err(anyhow!("Failed to read bytes into buf with reason: {err}")),
    }
}
