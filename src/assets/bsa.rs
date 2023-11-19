// COMMENT: https://en.uesp.net/wiki/Morrowind_Mod:BSA_File_Format and linked ghostwheel's site
use anyhow::{anyhow, Context, Result};
use fs_err::File;
use std::io::{Read, Seek, SeekFrom};

pub(crate) struct Bsa {
    pub(crate) path: String,
    pub(crate) names: Vec<String>,
    data_offset: u64,
    meta: Vec<u8>,
}

impl Bsa {
    pub(crate) fn new(path: &str) -> Result<Bsa> {
        let bsa = File::open(path).with_context(|| "Failed to open file for reading")?;
        let header = read_n_u8(&bsa, 12).with_context(|| "Failed to read header")?;
        let magic = four_u8_to_le_u32(&header, 0).with_context(|| "Failed to read magic number from header")? as usize;
        // COMMENT: le hex 0x00000100 = 256
        if magic != 256 {
            return Err(anyhow!(
                "Magic number(decimal) is wrong: expected \"{}\", got \"{}\". Program expects Morrowind's format BSA.",
                magic,
                256
            ));
        }
        let meta_size = four_u8_to_le_u32(&header, 4).with_context(|| "Failed to read meta size from header")? as usize;
        let num_files = four_u8_to_le_u32(&header, 8).with_context(|| "Failed to read files quantity from header")? as usize;
        let data_offset = 12 + meta_size + 8 * num_files;
        let meta = read_n_u8(&bsa, meta_size).with_context(|| "Failed to read meta")?;
        let mut names = Vec::with_capacity(num_files);
        let names_string = core::str::from_utf8(&meta[12 * num_files..])
            .with_context(|| "Failed to parse ANSI string with file names. File is probably corrupted.")?;
        names.extend(names_string.split('\0').map(|x| x.to_owned()));
        Ok(Bsa {
            path: path.to_owned(),
            names,
            data_offset: data_offset as u64,
            meta,
        })
    }

    pub(crate) fn get_file_by_index(&self, index: usize) -> Result<Vec<u8>> {
        let file_size = four_u8_to_le_u32(&self.meta, index * 8).with_context(|| "Failed to read file size from meta")? as u64;
        let file_offset = four_u8_to_le_u32(&self.meta, index * 8 + 4).with_context(|| "Failed to read file offset from meta")? as u64;
        let mut file = File::open(&self.path).with_context(|| format!("Failed to open bsa file \"{}\" for reading", &self.path))?;
        file.seek(SeekFrom::Start(file_offset + self.data_offset)).with_context(|| {
            format!(
                "Failed to start reading file from bsa file \"{}\" at offset \"{}\"",
                &self.path,
                file_offset + self.data_offset
            )
        })?;
        read_n_u8(&file, file_size as usize).with_context(|| "Failed to read file")
    }
}

fn four_u8_to_le_u32(buffer: &[u8], offset: usize) -> Result<u32> {
    let four_u8: [u8; 4] = buffer[offset..offset + 4]
        .try_into()
        .with_context(|| "Failed to get 4 bytes from buffer")?;
    Ok(u32::from_le_bytes(four_u8))
}

fn read_n_u8(file: &File, take: usize) -> Result<Vec<u8>> {
    let mut handle = file.take(take as u64);
    let mut buffer = Vec::with_capacity(take);
    match handle.read_to_end(&mut buffer) {
        Ok(num) => match num == take {
            true => Ok(buffer),
            false => Err(anyhow!(format!(
                "Failed to read bytes into buffer: expected \"{}\", got \"{}\"",
                take, num
            ))),
        },
        Err(err) => Err(anyhow!("Failed to read bytes into buf with reason: {}", err)),
    }
}
