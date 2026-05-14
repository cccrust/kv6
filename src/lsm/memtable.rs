use parking_lot::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

type KeyValue = (Vec<u8>, Vec<u8>);

#[allow(clippy::type_complexity)]
pub struct MemTable {
    size: AtomicUsize,
    max_size: usize,
    data: Arc<RwLock<Vec<KeyValue>>>,
}

impl MemTable {
    pub fn new(max_size: usize) -> Self {
        MemTable {
            size: AtomicUsize::new(0),
            max_size,
            data: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn put(&self, key: Vec<u8>, value: Vec<u8>) {
        let mut data = self.data.write();
        data.push((key.clone(), value));
        self.size.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let data = self.data.read();
        for (k, v) in data.iter().rev() {
            if k == key {
                return Some(v.clone());
            }
        }
        None
    }

    pub fn is_full(&self) -> bool {
        self.size.load(Ordering::Relaxed) >= self.max_size
    }

    pub fn flush(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut data = self.data.write();
        let mut result = std::mem::take(&mut *data);
        self.size.store(0, Ordering::Relaxed);
        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }

    pub fn len(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl MemTable {
    pub fn iter(&self) -> MemTableIterator {
        MemTableIterator {
            data: self.data.read().clone(),
            index: 0,
        }
    }
}

pub struct MemTableIterator {
    data: Vec<(Vec<u8>, Vec<u8>)>,
    index: usize,
}

impl Iterator for MemTableIterator {
    type Item = (Vec<u8>, Vec<u8>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.data.len() {
            let item = self.data[self.index].clone();
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memtable_put_get() {
        let mt = MemTable::new(100);
        mt.put(b"key1".to_vec(), b"value1".to_vec());
        mt.put(b"key2".to_vec(), b"value2".to_vec());

        assert_eq!(mt.get(b"key1"), Some(b"value1".to_vec()));
        assert_eq!(mt.get(b"key2"), Some(b"value2".to_vec()));
        assert_eq!(mt.get(b"key3"), None);
    }

    #[test]
    fn test_memtable_flush() {
        let mt = MemTable::new(100);
        mt.put(b"key1".to_vec(), b"value1".to_vec());
        mt.put(b"key2".to_vec(), b"value2".to_vec());

        let flushed = mt.flush();
        assert_eq!(flushed.len(), 2);
        assert!(mt.is_empty());
    }

    #[test]
    fn test_memtable_is_full() {
        let mt = MemTable::new(2);
        mt.put(b"key1".to_vec(), b"value1".to_vec());
        assert!(!mt.is_full());

        mt.put(b"key2".to_vec(), b"value2".to_vec());
        assert!(mt.is_full());
    }
}
