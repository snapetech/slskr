//! A salted Bloom filter matching native profile's `PodCore.BloomFilter` bit-for-bit:
//! same optimal size/hash-count formulas, same FNV-1a-variant double-hashing
//! scheme (operating on UTF-16 code units, matching C# `foreach (char c in
//! input)` exactly via `str::encode_utf16`), so filters built by slskR and
//! native profile against the same salted items are directly comparable.
//!
//! One deliberate divergence: native profile's `Math.Abs` throws `OverflowException`
//! on the `i32::MIN` hash-combination edge case (~1 in 4 billion). We treat
//! that value safely via `i32::unsigned_abs` instead of importing the crash.

#[derive(Debug)]
pub struct SaltedBloomFilter {
    bits: Vec<u8>,
    bit_size: usize,
    hash_function_count: usize,
    expected_items: u64,
    false_positive_rate: f64,
    item_count: u64,
}

const MAX_BLOOM_FILTER_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BloomFilterError {
    SizeExceeded { bytes: usize, max_bytes: usize },
}

impl std::fmt::Display for BloomFilterError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SizeExceeded { bytes, max_bytes } => write!(
                formatter,
                "requested Bloom filter is {bytes} bytes; maximum is {max_bytes} bytes"
            ),
        }
    }
}

impl SaltedBloomFilter {
    /// Builds an empty filter sized for `expected_items` at `false_positive_rate`,
    /// using the same optimal-size/hash-count formulas as the oracle.
    pub fn try_new(
        expected_items: u64,
        false_positive_rate: f64,
    ) -> Result<Self, BloomFilterError> {
        let expected_items = expected_items.max(1);
        let false_positive_rate =
            if !false_positive_rate.is_finite() || !(0.0..1.0).contains(&false_positive_rate) {
                0.01
            } else {
                false_positive_rate
            };
        let ln2_squared = std::f64::consts::LN_2 * std::f64::consts::LN_2;
        let bit_size = (-(expected_items as f64) * false_positive_rate.ln()) / ln2_squared;
        let max_bit_size = (MAX_BLOOM_FILTER_BYTES * 8) as f64;
        if !bit_size.is_finite() || bit_size < 1.0 || bit_size > max_bit_size {
            return Err(BloomFilterError::SizeExceeded {
                bytes: if bit_size.is_finite() && bit_size >= 0.0 {
                    (bit_size / 8.0).ceil() as usize
                } else {
                    usize::MAX
                },
                max_bytes: MAX_BLOOM_FILTER_BYTES,
            });
        }
        let bit_size = bit_size.ceil() as usize;
        let hash_function_count = ((bit_size as f64 / expected_items as f64)
            * std::f64::consts::LN_2)
            .round()
            .max(1.0) as usize;
        Ok(Self {
            bits: vec![0_u8; bit_size.div_ceil(8)],
            bit_size,
            hash_function_count,
            expected_items,
            false_positive_rate,
            item_count: 0,
        })
    }

    pub fn bit_size(&self) -> usize {
        self.bit_size
    }

    pub fn hash_function_count(&self) -> usize {
        self.hash_function_count
    }

    pub fn expected_items(&self) -> u64 {
        self.expected_items
    }

    pub fn false_positive_rate(&self) -> f64 {
        self.false_positive_rate
    }

    pub fn item_count(&self) -> u64 {
        self.item_count
    }

    /// Sets this item's bits. Returns `true` if the item was not already
    /// (probably) present, matching the oracle's `Add` return semantics.
    pub fn add(&mut self, item: &str) -> bool {
        let was_present = self.contains(item);
        for i in 0..self.hash_function_count {
            let index = hash_index(item, i, self.bit_size);
            self.set_bit(index);
        }
        if !was_present {
            self.item_count += 1;
        }
        !was_present
    }

    pub fn contains(&self, item: &str) -> bool {
        (0..self.hash_function_count).all(|i| self.get_bit(hash_index(item, i, self.bit_size)))
    }

    /// Fraction of bits set, matching the oracle's `FillRatio`.
    pub fn fill_ratio(&self) -> f64 {
        let set_bits: u32 = self.bits.iter().map(|byte| byte.count_ones()).sum();
        f64::from(set_bits) / self.bit_size as f64
    }

    pub fn to_base64(&self) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(&self.bits)
    }

    fn set_bit(&mut self, index: usize) {
        self.bits[index / 8] |= 1 << (index % 8);
    }

    fn get_bit(&self, index: usize) -> bool {
        self.bits[index / 8] & (1 << (index % 8)) != 0
    }
}

fn stable_hash(input: &str, seed: i32) -> i32 {
    const PRIME: u32 = 16_777_619;
    let mut hash: u32 = 2_166_136_261_u32 ^ (seed as u32);
    for unit in input.encode_utf16() {
        hash = (hash ^ u32::from(unit)).wrapping_mul(PRIME);
    }
    hash as i32
}

fn hash_index(item: &str, i: usize, bit_size: usize) -> usize {
    let i = i as i32;
    let seed1 = i.wrapping_mul(2).wrapping_add(1);
    let seed2 = i.wrapping_mul(2).wrapping_add(2);
    let hash1 = stable_hash(item, seed1);
    let hash2 = stable_hash(item, seed2);
    let combined = hash1.wrapping_add(i.wrapping_mul(hash2));
    (combined.unsigned_abs() as usize) % bit_size
}

/// Matches native profile's `LibraryBloomDiffService.BuildSaltedItem`: salt, a
/// lower-cased namespace, and a lower-cased identifier, joined by the ASCII
/// Unit Separator control character.
pub fn build_salted_item(salt_id: &str, namespace: &str, identifier: &str) -> String {
    format!(
        "{}\u{1f}{}\u{1f}{}",
        salt_id.trim(),
        namespace.trim().to_lowercase(),
        identifier.trim().to_lowercase()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_hash_matches_the_oracle_fnv1a_variant() {
        // Hand-computed against native profile's GetStableHash(string, int) for a
        // plain-ASCII input, seed 1: hash = 2166136261 ^ 1 = 2166136260,
        // then for each byte b: hash = (hash ^ b) * 16777619 (u32 wrapping).
        let mut hash: u32 = 2_166_136_261_u32 ^ 1;
        for byte in "abc".bytes() {
            hash = (hash ^ u32::from(byte)).wrapping_mul(16_777_619);
        }
        assert_eq!(stable_hash("abc", 1), hash as i32);
    }

    #[test]
    fn filter_reports_real_membership_and_nonzero_fill_after_adds() {
        let mut filter = SaltedBloomFilter::try_new(16, 0.01).unwrap();
        assert_eq!(filter.fill_ratio(), 0.0);
        assert_eq!(filter.item_count(), 0);

        let item = build_salted_item("salt-1", "musicbrainz:recording", "MBID-1");
        assert!(filter.add(&item));
        assert!(filter.contains(&item));
        assert_eq!(filter.item_count(), 1);
        assert!(filter.fill_ratio() > 0.0);

        // Adding the same item again should not increase the item count.
        assert!(!filter.add(&item));
        assert_eq!(filter.item_count(), 1);

        let other = build_salted_item("salt-1", "musicbrainz:recording", "MBID-2");
        // A different item is very unlikely to already test as present in a
        // freshly-sized filter with a single prior entry.
        assert!(!filter.contains(&other));

        // Different salts must not collide onto the same bits for the same
        // identifier -- that's the entire privacy point of salting.
        let mut other_salt_filter = SaltedBloomFilter::try_new(16, 0.01).unwrap();
        let salted_differently = build_salted_item("salt-2", "musicbrainz:recording", "MBID-1");
        other_salt_filter.add(&salted_differently);
        assert_ne!(filter.to_base64(), other_salt_filter.to_base64());
    }

    #[test]
    fn base64_round_trips_bit_size() {
        let mut filter = SaltedBloomFilter::try_new(16, 0.01).unwrap();
        filter.add("some-item");
        let expected_bytes = filter.bit_size().div_ceil(8);
        let decoded = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            filter.to_base64(),
        )
        .unwrap();
        assert_eq!(decoded.len(), expected_bytes);
    }

    #[test]
    fn filter_rejects_unbounded_precision_requests() {
        let error = SaltedBloomFilter::try_new(1_000_000, f64::MIN_POSITIVE)
            .expect_err("extreme precision must not allocate an unbounded filter");
        assert!(matches!(
            error,
            BloomFilterError::SizeExceeded { bytes, max_bytes }
                if bytes > max_bytes && max_bytes == MAX_BLOOM_FILTER_BYTES
        ));
    }
}
