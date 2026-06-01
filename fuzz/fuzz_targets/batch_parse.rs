#![no_main]

use atomwrite::commands::batch::BatchOp;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        for line in s.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let _ = serde_json::from_str::<BatchOp>(trimmed);
        }
    }
});
