#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() >= 8 {
        let bytes = &data[..8];
        let raw = u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        let id = ax_id::Id(raw);
        let _ = id.timestamp_ms();
        let _ = id.node_id();
        let _ = id.raw_sequence();
    }
});
