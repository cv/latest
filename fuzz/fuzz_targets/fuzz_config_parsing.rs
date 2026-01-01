#![no_main]

use libfuzzer_sys::fuzz_target;
use latest::config::Config;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz TOML config parsing
        // The config parser should gracefully handle all inputs
        let _: Result<Config, _> = toml::from_str(s);

        // Even invalid TOML shouldn't crash - we just get parse errors
    }
});
