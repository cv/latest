#![no_main]

use libfuzzer_sys::fuzz_target;
use latest::{is_newer, sources::extract_version};

fuzz_target!(|data: &[u8]| {
    // Try to parse as UTF-8 string
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz version extraction
        let _ = extract_version(s);

        // If we can extract a version, also test comparison
        if let Some(v1) = extract_version(s) {
            // Test comparing with itself (should never be newer)
            let _ = is_newer(&v1, &v1);

            // Test comparing with common versions
            let _ = is_newer(&v1, "1.0.0");
            let _ = is_newer("1.0.0", &v1);
            let _ = is_newer(&v1, "0.0.1");
            let _ = is_newer(&v1, "999.999.999");
        }

        // Test is_newer with raw input split in half
        if s.len() >= 2 {
            let mid = s.len() / 2;
            let (left, right) = s.split_at(mid);
            let _ = is_newer(left, right);
            let _ = is_newer(right, left);
        }
    }
});
