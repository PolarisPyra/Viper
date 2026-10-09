//! Bounded block cache with logical seeks, so tag parsers can revisit remote data.
use std::{
    collections::VecDeque,
    io::{self, Read, Seek, SeekFrom},
};

const BLOCK_SIZE: usize = 128 * 1024;
const MAX_BLOCKS: usize = 16;

pub(super) struct SmbReader<R> {
    inner: R,
    position: u64,
    length: Option<u64>,
    blocks: VecDeque<(u64, Vec<u8>)>,
}

impl<R> SmbReader<R> {
    pub(super) fn new(inner: R) -> Self {
        Self {
            inner,
            position: 0,
            length: None,
            blocks: VecDeque::new(),
        }
    }
}

impl<R: Read + Seek> Read for SmbReader<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() || self.length.is_some_and(|length| self.position >= length) {
            return Ok(0);
        }
        let start = self.position / BLOCK_SIZE as u64 * BLOCK_SIZE as u64;
        let index = if let Some(index) = self.blocks.iter().position(|(offset, _)| *offset == start)
        {
            index
        } else {
            self.inner.seek(SeekFrom::Start(start))?;
            let mut data = vec![0; BLOCK_SIZE];
            let mut filled = 0;
            while filled < data.len() {
                match self.inner.read(&mut data[filled..]) {
                    Ok(0) => {
                        self.length = Some(if filled == 0 {
                            // A caller may seek beyond EOF; that position is not the file length.
                            self.inner.seek(SeekFrom::End(0))?
                        } else {
                            start + filled as u64
                        });
                        break;
                    }
                    Ok(count) => filled += count,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => return Err(error),
                }
            }
            data.truncate(filled);
            if self.blocks.len() == MAX_BLOCKS {
                self.blocks.pop_front();
            }
            self.blocks.push_back((start, data));
            self.blocks.len() - 1
        };
        let block = self.blocks.remove(index).expect("cached block exists");
        let offset = (self.position - start) as usize;
        let count = output.len().min(block.1.len().saturating_sub(offset));
        if count > 0 {
            output[..count].copy_from_slice(&block.1[offset..offset + count]);
            self.position += count as u64;
        }
        self.blocks.push_back(block);
        Ok(count)
    }
}

impl<R: Seek> Seek for SmbReader<R> {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let position = match from {
            SeekFrom::Start(position) => Some(position),
            SeekFrom::Current(offset) => self.position.checked_add_signed(offset),
            SeekFrom::End(offset) => {
                let length = match self.length {
                    Some(length) => length,
                    None => {
                        let length = self.inner.seek(SeekFrom::End(0))?;
                        self.length = Some(length);
                        length
                    }
                };
                length.checked_add_signed(offset)
            }
        }
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid SMB seek"))?;
        self.position = position;
        Ok(position)
    }
}
