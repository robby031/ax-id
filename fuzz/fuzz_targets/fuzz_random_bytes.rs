#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() == 8 {
        let raw = u64::from_le_bytes([
            data[0], data[1], data[2], data[3],
            data[4], data[5], data[6], data[7],
        ]);
        let id = ax_id::Id(raw);
        let ts = id.timestamp_ms();
        let node = id.node_id();
        let seq = id.raw_sequence();

        assert_eq!(id.0, (ts << 23) | ((node as u64) << 13) | (seq as u64));
    }
});
