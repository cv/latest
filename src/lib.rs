// Library crate exposing internals for benchmarking

pub mod cache;
pub mod config;
pub mod project;
pub mod sources;

/// Check if `latest` is a newer version than `installed`.
/// Compares numeric version components.
#[must_use]
pub fn is_newer(installed: &str, latest: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.split(|c: char| !c.is_ascii_digit()).filter_map(|s| s.parse().ok()).collect()
    };
    let (a, b) = (parse(installed), parse(latest));
    (0..a.len().max(b.len())).any(|i| {
        let (x, y) = (*a.get(i).unwrap_or(&0), *b.get(i).unwrap_or(&0));
        x < y && (0..i).all(|j| a.get(j) == b.get(j))
    })
}

/// Parse a package argument, extracting optional source prefix.
/// e.g., "npm:express" -> (Some("npm"), "express")
///       "express" -> (None, "express")
#[must_use]
pub fn parse_package_arg(arg: &str) -> (Option<String>, String) {
    if let Some((prefix, rest)) = arg.split_once(':') {
        // Only treat as source prefix if it's a known source name
        if sources::source_by_name(prefix).is_some() {
            return (Some(prefix.to_string()), rest.to_string());
        }
    }
    (None, arg.to_string())
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating realistic version strings
    fn version_strategy() -> impl Strategy<Value = String> {
        "[0-9]{1,3}(\\.[0-9]{1,3}){0,3}(-[a-zA-Z0-9]+)?"
    }

    proptest! {
        // is_newer should never panic on any input (including garbage)
        #[test]
        fn is_newer_never_panics(a in ".*", b in ".*") {
            let _ = is_newer(&a, &b);
        }

        // is_newer is irreflexive: x is never newer than itself
        #[test]
        fn is_newer_irreflexive(v in version_strategy()) {
            prop_assert!(!is_newer(&v, &v));
        }

        // is_newer is asymmetric: if a < b then !(b < a)
        #[test]
        fn is_newer_asymmetric(a in version_strategy(), b in version_strategy()) {
            if is_newer(&a, &b) {
                prop_assert!(!is_newer(&b, &a));
            }
        }

        // is_newer is transitive: if a < b and b < c then a < c
        #[test]
        fn is_newer_transitive(
            a in version_strategy(),
            b in version_strategy(),
            c in version_strategy()
        ) {
            if is_newer(&a, &b) && is_newer(&b, &c) {
                prop_assert!(is_newer(&a, &c), "transitivity violated: {} < {} < {} but not {} < {}", a, b, c, a, c);
            }
        }

        // Incrementing any component makes a newer version
        #[test]
        fn is_newer_increment_is_newer(
            major in 0u32..100,
            minor in 0u32..100,
            patch in 0u32..100,
            incr in 1u32..10
        ) {
            let base = format!("{}.{}.{}", major, minor, patch);
            let newer_patch = format!("{}.{}.{}", major, minor, patch + incr);
            let newer_minor = format!("{}.{}.{}", major, minor + incr, patch);
            let newer_major = format!("{}.{}.{}", major + incr, minor, patch);

            prop_assert!(is_newer(&base, &newer_patch), "{} should be older than {}", base, newer_patch);
            prop_assert!(is_newer(&base, &newer_minor), "{} should be older than {}", base, newer_minor);
            prop_assert!(is_newer(&base, &newer_major), "{} should be older than {}", base, newer_major);
        }

        // parse_package_arg never panics
        #[test]
        fn parse_package_arg_never_panics(s in "\\PC*") {
            let _ = parse_package_arg(&s);
        }

        // parse_package_arg: if no source returned, package equals input
        #[test]
        fn parse_package_arg_no_source_preserves_input(s in "[a-zA-Z0-9_-]+") {
            let (source, pkg) = parse_package_arg(&s);
            if source.is_none() {
                prop_assert_eq!(pkg, s);
            }
        }

        // parse_package_arg: if source returned, input must have had a colon
        #[test]
        fn parse_package_arg_source_requires_colon(s in "\\PC*") {
            let (source, _) = parse_package_arg(&s);
            if source.is_some() {
                prop_assert!(s.contains(':'));
            }
        }

        // extract_version never panics
        #[test]
        fn extract_version_never_panics(s in "\\PC*") {
            let _ = sources::extract_version(&s);
        }

        // extract_version: output (if Some) should be parseable as version-like
        #[test]
        fn extract_version_output_is_valid(s in "\\PC*") {
            if let Some(v) = sources::extract_version(&s) {
                // Should contain at least one digit
                prop_assert!(v.chars().any(|c| c.is_ascii_digit()));
                // Should match pattern: digits possibly with dots/dashes/alphanumerics
                prop_assert!(v.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-'));
            }
        }

        // extract_version: extracted versions work with is_newer without panic
        #[test]
        fn extract_version_output_works_with_is_newer(s in "\\PC*") {
            if let Some(v) = sources::extract_version(&s) {
                // Should be usable in is_newer without panic
                let _ = is_newer(&v, "1.0.0");
                let _ = is_newer("1.0.0", &v);
                // Reflexive property should hold for extracted versions
                prop_assert!(!is_newer(&v, &v));
            }
        }

        // source_by_name never panics
        #[test]
        fn source_by_name_never_panics(s in "\\PC*") {
            let _ = sources::source_by_name(&s);
        }
    }
}
