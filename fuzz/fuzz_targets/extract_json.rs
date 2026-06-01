#![no_main]

use atomwrite::commands::extract::check_depth;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(s) {
            let _ = check_depth(&value, 128);
        }
    }
});
