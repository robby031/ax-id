use core::fmt;

use crate::internal::MAX_NODE_ID;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdError {
    ClockSkew { elapsed_ms: u64 },
    SequenceExhausted,
    InvalidNodeId(u16),
    InvalidFormat,
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdError::ClockSkew { elapsed_ms } => {
                write!(f, "clock skew detected: {} ms backward", elapsed_ms)
            }
            IdError::SequenceExhausted => {
                write!(f, "sequence exhausted for this millisecond")
            }
            IdError::InvalidNodeId(id) => {
                write!(f, "node id {} exceeds maximum {}", id, MAX_NODE_ID)
            }
            IdError::InvalidFormat => write!(f, "invalid ID format"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for IdError {}
