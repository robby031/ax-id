pub(crate) const CUSTOM_EPOCH_MS: u64 = 1_704_067_200_000;
pub(crate) const TIMESTAMP_BITS: u64 = 40;
pub(crate) const NODE_BITS: u64 = 10;
pub(crate) const SEQUENCE_BITS: u64 = 13;

pub(crate) const TIMESTAMP_SHIFT: u64 = NODE_BITS + SEQUENCE_BITS;
pub(crate) const NODE_SHIFT: u64 = SEQUENCE_BITS;
pub(crate) const SEQUENCE_MASK: u64 = (1 << SEQUENCE_BITS) - 1;
pub(crate) const MAX_SEQUENCE: u64 = (1 << SEQUENCE_BITS) - 1;
pub(crate) const MAX_NODE_ID: u16 = (1 << NODE_BITS) - 1;

#[cfg(feature = "std")]
pub use std_dependent::*;

#[cfg(feature = "std")]
mod std_dependent {
    use alloc::sync::Arc;
    use std::sync::OnceLock;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::error::IdError;

    #[inline]
    pub(crate) fn current_timestamp_ms() -> Result<u64, IdError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| IdError::ClockSkew {
                elapsed_ms: e.duration().as_millis() as u64,
            })?;
        let ms = now.as_millis() as u64;
        if ms < super::CUSTOM_EPOCH_MS {
            return Err(IdError::ClockSkew {
                elapsed_ms: super::CUSTOM_EPOCH_MS - ms,
            });
        }
        Ok(ms - super::CUSTOM_EPOCH_MS)
    }

    #[inline]
    pub(crate) fn thread_local_entropy() -> u64 {
        ax_rnd::u64()
    }

    pub(crate) fn resolve_node_id() -> u16 {
        if let Ok(env_node) = std::env::var("AX_ID_NODE") {
            match env_node.parse::<u16>() {
                Ok(id) if id <= super::MAX_NODE_ID => return id,
                _ => {
                    eprintln!(
                        "ax-id: AX_ID_NODE={:?} is not a valid node id (0..={}); \
                         falling back to random node id",
                        env_node,
                        super::MAX_NODE_ID,
                    );
                }
            }
        }

        (ax_rnd::u64() & (super::MAX_NODE_ID as u64)) as u16
    }

    pub fn global_generator() -> Arc<crate::generator::Generator> {
        static GEN: OnceLock<Arc<crate::generator::Generator>> = OnceLock::new();
        GEN.get_or_init(|| Arc::new(crate::generator::Generator::new_auto()))
            .clone()
    }
}
