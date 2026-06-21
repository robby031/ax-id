use core::fmt;
use core::str::FromStr;

use crate::internal::{NODE_BITS, SEQUENCE_BITS, SEQUENCE_MASK, TIMESTAMP_BITS};
#[cfg(feature = "std")]
use crate::internal::CUSTOM_EPOCH_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[cfg_attr(
    feature = "zerocopy",
    derive(
        zerocopy::IntoBytes,
        zerocopy::FromBytes,
        zerocopy::Immutable,
        zerocopy::KnownLayout
    )
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct Id(pub u64);

impl Id {
    #[inline]
    pub const fn timestamp_ms(&self) -> u64 {
        let timestamp_shift = NODE_BITS + SEQUENCE_BITS;
        (self.0 >> timestamp_shift) & ((1 << TIMESTAMP_BITS) - 1)
    }

    #[inline]
    pub const fn node_id(&self) -> u16 {
        let node_shift = SEQUENCE_BITS;
        ((self.0 >> node_shift) & ((1 << NODE_BITS) - 1)) as u16
    }

    #[inline]
    pub const fn raw_sequence(&self) -> u16 {
        (self.0 & SEQUENCE_MASK) as u16
    }

    #[inline]
    #[cfg(feature = "std")]
    pub fn datetime(&self) -> std::time::SystemTime {
        std::time::UNIX_EPOCH
            + std::time::Duration::from_millis(CUSTOM_EPOCH_MS + self.timestamp_ms())
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = [0u8; 16];
        let mut n = self.0;
        for i in (0..16).rev() {
            buf[i] = hex_digit((n & 0xf) as u8);
            n >>= 4;
        }
        core::str::from_utf8(&buf)
            .map(|s| f.write_str(s))
            .unwrap_or(Err(fmt::Error))
    }
}

impl FromStr for Id {
    type Err = crate::error::IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            return u64::from_str_radix(rest, 16)
                .map(Id)
                .map_err(|_| crate::error::IdError::InvalidFormat);
        }

        if s.len() == 16 && s.bytes().all(|b| b.is_ascii_hexdigit()) {
            return u64::from_str_radix(s, 16)
                .map(Id)
                .map_err(|_| crate::error::IdError::InvalidFormat);
        }

        if let Ok(v) = s.parse::<u64>() {
            return Ok(Id(v));
        }

        u64::from_str_radix(s, 16)
            .map(Id)
            .map_err(|_| crate::error::IdError::InvalidFormat)
    }
}

impl From<Id> for u64 {
    #[inline]
    fn from(id: Id) -> Self {
        id.0
    }
}

impl From<u64> for Id {
    #[inline]
    fn from(value: u64) -> Self {
        Id(value)
    }
}

#[inline]
const fn hex_digit(n: u8) -> u8 {
    if n < 10 { b'0' + n } else { b'a' + (n - 10) }
}
