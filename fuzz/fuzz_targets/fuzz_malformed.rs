#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let trimmed = s.trim();
        let _ = trimmed.parse::<ax_id::Id>();
        let _ = trimmed.parse::<u64>();
        let _ = u64::from_str_radix(trimmed, 16);
    }
});
