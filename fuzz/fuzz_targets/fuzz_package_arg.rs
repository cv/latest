#![no_main]

use libfuzzer_sys::fuzz_target;
use latest::parse_package_arg;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz the package argument parser
        // This should handle all inputs gracefully without panicking
        let (source, package) = parse_package_arg(s);

        // Verify invariants:
        // - If source is Some, the original string should have contained a ':'
        if source.is_some() {
            assert!(s.contains(':'));
        }

        // - The package should never be empty if input was non-empty
        // (unless input was just a known source prefix with colon, which gives empty package)
        // Actually empty package is valid for "npm:" etc.

        // - If no source prefix, package should equal original input
        if source.is_none() {
            assert_eq!(package, s);
        }
    }
});
