use std::fs::{File, OpenOptions};
use std::io::Write;

use super::bloom::BloomFilter;
use super::sstable::IndexEntry;

pub struct SSTableBuilder {
    #[allow(dead_code)]
    path: String,
    file: Option<File>,
    #[allow(dead_code)]
    index: Vec<IndexEntry>,
    bloom_filter: BloomFilter,
    current_offset: u32,
}

impl SSTableBuilder {
    pub fn new(path: &str) -> Self {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .ok();

        SSTableBuilder {
            path: path.to_string(),
            file,
            index: Vec::new(),
            bloom_filter: BloomFilter::new(10000, 0.01),
            current_offset: 0,
        }
    }

    #[allow(clippy::io_other_error, clippy::unnecessary_cast)]
    pub fn build(&mut self, data: &[(Vec<u8>, Vec<u8>)]) -> std::io::Result<()> {
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| std::io::Error::other("File not initialized"))?;

        let mut index_data = Vec::new();

        for (key, value) in data {
            let value_offset = self.current_offset;
            let value_size = value.len() as u32;

            file.write_all(value)?;
            self.current_offset += value.len() as u32;

            index_data.extend_from_slice(&(key.len() as u16).to_le_bytes());
            index_data.extend_from_slice(key);
            index_data.extend_from_slice(&value_offset.to_le_bytes());
            index_data.extend_from_slice(&value_size.to_le_bytes());

            self.bloom_filter.add(key);
        }

        let index_offset = self.current_offset;
        let index_size = index_data.len() as u32;
        file.write_all(&index_data)?;
        self.current_offset += index_size;

        let bloom_offset = self.current_offset;
        let bloom_data = self.bloom_filter.to_bytes();
        let bloom_size = bloom_data.len() as u32;
        file.write_all(&bloom_data)?;
        self.current_offset += bloom_size;

        let mut header = [0u8; 16];
        header[0..4].copy_from_slice(&index_offset.to_le_bytes());
        header[4..8].copy_from_slice(&index_size.to_le_bytes());
        header[8..12].copy_from_slice(&bloom_offset.to_le_bytes());
        header[12..16].copy_from_slice(&bloom_size.to_le_bytes());

        file.write_all(&header)?;
        file.flush()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sstable_builder() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_builder.sst");
        let path_str = path.to_str().unwrap();

        let data = vec![
            (b"aaa".to_vec(), b"111".to_vec()),
            (b"bbb".to_vec(), b"222".to_vec()),
            (b"ccc".to_vec(), b"333".to_vec()),
        ];

        let mut builder = SSTableBuilder::new(path_str);
        builder.build(&data).unwrap();

        assert!(path.exists());

        std::fs::remove_file(path).ok();
    }
}
