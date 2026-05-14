use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const MURMUR_MIX: u64 = 0xc6a4a7935bd1e995;

pub struct BloomFilter {
    bits: Vec<u64>,
    m: usize,
    k: usize,
}

impl BloomFilter {
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        let m = Self::optimal_m(expected_items, false_positive_rate);
        let k = Self::optimal_k(m, expected_items);
        let bits = vec![0u64; m.div_ceil(64)];

        BloomFilter { bits, m, k }
    }

    fn optimal_m(n: usize, p: f64) -> usize {
        if n == 0 {
            return 64;
        }
        let m = (-(n as f64) * p.ln() / (2.0_f64.ln().powi(2))).ceil() as usize;
        m.max(64)
    }

    fn optimal_k(m: usize, n: usize) -> usize {
        if n == 0 {
            return 1;
        }
        let k = ((m as f64 / n as f64) * 2.0_f64.ln()).round() as usize;
        k.clamp(1, 16)
    }

    pub fn add(&mut self, key: &[u8]) {
        for i in 0..self.k {
            let hash = self.hash(key, i);
            let idx = hash % self.m;
            self.bits[idx / 64] |= 1 << (idx % 64);
        }
    }

    pub fn might_contain(&self, key: &[u8]) -> bool {
        for i in 0..self.k {
            let hash = self.hash(key, i);
            let idx = hash % self.m;
            if self.bits[idx / 64] & (1 << (idx % 64)) == 0 {
                return false;
            }
        }
        true
    }

    fn hash(&self, key: &[u8], seed: usize) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        seed.hash(&mut hasher);
        (hasher.finish() as usize) ^ (MURMUR_MIX as usize)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&(self.m as u32).to_le_bytes());
        data.extend_from_slice(&(self.k as u32).to_le_bytes());
        for &bits_entry in &self.bits {
            data.extend_from_slice(&bits_entry.to_le_bytes());
        }
        data
    }

    #[allow(clippy::manual_div_ceil)]
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }
        let m = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let k = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let bits_len = m.div_ceil(64);
        if data.len() < 8 + bits_len * 8 {
            return None;
        }
        let mut bits = Vec::with_capacity(bits_len);
        for i in 0..bits_len {
            let offset = 8 + i * 8;
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&data[offset..offset + 8]);
            bits.push(u64::from_le_bytes(bytes));
        }
        Some(BloomFilter { bits, m, k })
    }
}

impl Default for BloomFilter {
    fn default() -> Self {
        Self::new(10000, 0.01)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_filter_add_contains() {
        let mut bf = BloomFilter::new(100, 0.1);
        bf.add(b"key1");
        bf.add(b"key2");
        bf.add(b"key3");

        assert!(bf.might_contain(b"key1"));
        assert!(bf.might_contain(b"key2"));
        assert!(bf.might_contain(b"key3"));
        assert!(!bf.might_contain(b"key4"));
    }

    #[test]
    fn test_bloom_filter_serialization() {
        let mut bf = BloomFilter::new(100, 0.1);
        bf.add(b"test_key");

        let bytes = bf.to_bytes();
        let bf2 = BloomFilter::from_bytes(&bytes).unwrap();

        assert!(bf2.might_contain(b"test_key"));
    }
}
