//! Key package versions for Debian releases.
include!(concat!(env!("OUT_DIR"), "/key_package_versions.rs"));

#[cfg(test)]
mod tests {
    #[test]
    fn test_debhelper_versions() {
        assert!(super::debhelper_versions.get("sid").is_some());
        assert!(super::debhelper_versions.get("trixie").is_some());
    }
}
