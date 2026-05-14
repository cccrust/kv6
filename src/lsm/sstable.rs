use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use super::bloom::BloomFilter;

const HEADER_SIZE: usize = 16;

#[allow(dead_code)]
pub struct SSTable {
    path: String,
    file_size: u64,
    index: Vec<IndexEntry>,
    bloom_filter: BloomFilter,
}

pub struct IndexEntry {
    pub key: Vec<u8>,
    pub offset: u32,
    pub size: u32,
}

impl SSTable {
    pub fn open(path: &str) -> std::io::Result<Self> {
        let mut file = File::open(path)?;
        let metadata = file.metadata()?;
        let file_size = metadata.len();

        file.seek(SeekFrom::End(-(HEADER_SIZE as i64)))?;
        let mut header = [0u8; HEADER_SIZE];
        file.read_exact(&mut header)?;

        let index_offset = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        let index_size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
        let bloom_offset = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
        let bloom_size = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);

        file.seek(SeekFrom::Start(index_offset as u64))?;
        let mut index_data = vec![0u8; index_size as usize];
        file.read_exact(&mut index_data)?;

        let mut index = Vec::new();
        let mut pos = 0;
        while pos < index_data.len() {
            let key_len = u16::from_le_bytes([index_data[pos], index_data[pos + 1]]) as usize;
            pos += 2;
            let key = index_data[pos..pos + key_len].to_vec();
            pos += key_len;
            let offset = u32::from_le_bytes([
                index_data[pos],
                index_data[pos + 1],
                index_data[pos + 2],
                index_data[pos + 3],
            ]);
            pos += 4;
            let size = u32::from_le_bytes([
                index_data[pos],
                index_data[pos + 1],
                index_data[pos + 2],
                index_data[pos + 3],
            ]);
            pos += 4;
            index.push(IndexEntry { key, offset, size });
        }

        file.seek(SeekFrom::Start(bloom_offset as u64))?;
        let mut bloom_data = vec![0u8; bloom_size as usize];
        file.read_exact(&mut bloom_data)?;
        let bloom_filter = BloomFilter::from_bytes(&bloom_data).unwrap_or_default();

        Ok(SSTable {
            path: path.to_string(),
            file_size,
            index,
            bloom_filter,
        })
    }

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        if !self.bloom_filter.might_contain(key) {
            return None;
        }

        let entry = self.index.binary_search_by(|e| e.key.as_slice().cmp(key));
        if let Ok(idx) = entry {
            let entry = &self.index[idx];
            if let Ok(mut file) = File::open(&self.path) {
                if file.seek(SeekFrom::Start(entry.offset as u64)).is_ok() {
                    let mut data = vec![0u8; entry.size as usize];
                    if file.read_exact(&mut data).is_ok() {
                        return Some(data);
                    }
                }
            }
        }
        None
    }

    pub fn range_scan(&self, start: &[u8], end: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut results = Vec::new();
        for entry in &self.index {
            if entry.key.as_slice() >= start && entry.key.as_slice() <= end {
                if let Ok(mut file) = File::open(&self.path) {
                    file.seek(SeekFrom::Start(entry.offset as u64)).ok();
                    let mut data = vec![0u8; entry.size as usize];
                    if file.read_exact(&mut data).is_ok() {
                        results.push((entry.key.clone(), data));
                    }
                }
            }
        }
        results
    }

    pub fn get_bloom_filter(&self) -> &BloomFilter {
        &self.bloom_filter
    }

    pub fn get_all_keys(&self) -> Vec<Vec<u8>> {
        self.index.iter().map(|e| e.key.clone()).collect()
    }
}

pub struct SSTableReader {
    ss_tables: Vec<SSTable>,
}

impl SSTableReader {
    pub fn new() -> Self {
        SSTableReader {
            ss_tables: Vec::new(),
        }
    }

    pub fn add_sstable(&mut self, ss_table: SSTable) {
        self.ss_tables.push(ss_table);
    }

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        for ss in self.ss_tables.iter().rev() {
            if let Some(value) = ss.get(key) {
                return Some(value);
            }
        }
        None
    }

    pub fn range_scan(&self, start: &[u8], end: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut results = Vec::new();
        for ss in &self.ss_tables {
            results.extend(ss.range_scan(start, end));
        }
        results.sort_by(|a, b| a.0.cmp(&b.0));
        results
    }
}

impl Default for SSTableReader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::lsm::{SSTable, SSTableBuilder};

    #[test]
    fn test_sstable_build_and_read() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_sstable.sst");
        let path_str = path.to_str().unwrap();

        let data = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
            (b"key3".to_vec(), b"value3".to_vec()),
        ];

        let mut builder = SSTableBuilder::new(path_str);
        builder.build(&data).unwrap();

        let ss = SSTable::open(path_str).unwrap();

        assert_eq!(ss.get(b"key1"), Some(b"value1".to_vec()));
        assert_eq!(ss.get(b"key2"), Some(b"value2".to_vec()));
        assert_eq!(ss.get(b"key3"), Some(b"value3".to_vec()));
        assert_eq!(ss.get(b"key4"), None);

        std::fs::remove_file(path).ok();
    }
}
