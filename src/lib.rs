#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod error;
mod id;
mod internal;

#[cfg(feature = "std")]
mod generator;

#[cfg(any(
    feature = "serde",
    feature = "bytemuck",
    feature = "zerocopy",
    feature = "arbitrary",
    feature = "rkyv",
    feature = "borsh",
    feature = "sqlx",
    feature = "diesel",
    feature = "sea-orm"
))]
pub(crate) mod integrations;

#[cfg(feature = "serde")]
pub mod serde {
    //! Serde helpers for `Id`.
    pub use crate::integrations::serde_impl::hex;
}

pub use error::IdError;
pub use id::Id;

#[cfg(feature = "std")]
pub use generator::Generator;
#[cfg(feature = "std")]
pub use internal::global_generator;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniqueness_single_thread() {
        let generator = Generator::new(1).unwrap();
        let mut set = alloc::collections::BTreeSet::new();
        for _ in 0..100_000 {
            let id = generator.generate_simple();
            assert!(set.insert(id.0), "duplicate ID detected");
        }
    }

    #[test]
    fn monotonicity_same_millisecond() {
        let generator = Generator::new(1).unwrap();
        let mut prev = generator.generate_simple();
        for _ in 0..1_000 {
            let curr = generator.generate_simple();
            assert!(curr.0 > prev.0, "IDs must be monotonically increasing");
            prev = curr;
        }
    }

    #[test]
    fn node_id_extraction() {
        let generator = Generator::new(42).unwrap();
        let id = generator.generate_simple();
        assert_eq!(id.node_id(), 42);
    }

    #[test]
    fn display_hex() {
        let id = Id(0x08b53edc41582000);
        assert_eq!(format!("{}", id), "08b53edc41582000");
    }

    #[test]
    fn from_str_hex() {
        let id: Id = "08b53edc41582000".parse().unwrap();
        assert_eq!(id.0, 0x08b53edc41582000);
    }

    #[test]
    fn from_str_decimal() {
        let id: Id = "123456789012345".parse().unwrap();
        assert_eq!(id.0, 123456789012345);
    }

    #[test]
    fn parse_display_roundtrip() {
        let original = Id(0xdeadbeefcafe0000);
        let parsed: Id = original.to_string().parse().unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_roundtrip_json() {
        use serde_json;
        let original = Id(0x08b53edc41582000);
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Id = serde_json::from_str(&json).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_hex_roundtrip() {
        use serde_json;
        let original = Id(0x08b53edc41582000);
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Id = serde_json::from_str(&json).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn lexicographic_ordering_matches_raw() {
        let a = Id(0x1000000000000000);
        let b = Id(0x2000000000000000);
        assert_eq!(a.0 < b.0, a < b);
        assert_eq!(a.0.cmp(&b.0), a.cmp(&b));
    }

    #[test]
    fn no_duplicate_generation_across_sequences() {
        let generator = Generator::new(1).unwrap();
        let mut set = alloc::collections::BTreeSet::new();
        for _ in 0..10_000 {
            let id = generator.generate().unwrap();
            assert!(set.insert(id.0), "duplicate across sequences");
        }
    }

    #[test]
    fn bit_extraction_consistency() {
        let id = Id((1234 << 23) | (42 << 13) | 7);
        assert_eq!(id.timestamp_ms(), 1234);
        assert_eq!(id.node_id(), 42);
        assert_eq!(id.raw_sequence(), 7);
    }

    #[test]
    fn parse_display_roundtrip_all_digit_hex() {
        let cases = [
            Id(0x0000_0000_1000_0000),
            Id(0x1234_5678_9012_3456),
            Id(0x0123_4567_8901_2345),
            Id(0),
        ];
        for original in cases {
            let s = original.to_string();
            let parsed: Id = s.parse().unwrap();
            assert_eq!(original, parsed, "roundtrip failed for {}", s);
        }
    }

    #[test]
    fn simple_no_collision_under_exhaustion() {
        use alloc::collections::BTreeSet;
        let g = Generator::new(1).unwrap();
        let mut set = BTreeSet::new();

        for _ in 0..200_000 {
            let id = g.generate_simple();
            assert!(set.insert(id.0), "duplicate from generate_simple");
        }
    }

    #[test]
    fn generate_new_ms_starts_at_zero() {
        let g = Generator::new(1).unwrap();
        // Burn through any same-ms batch left over from construction.
        std::thread::sleep(std::time::Duration::from_millis(2));
        let id = g.generate().unwrap();
        assert_eq!(
            id.raw_sequence(),
            0,
            "new-ms batch should start at seq 0, got {}",
            id.raw_sequence()
        );
    }
}
