//! Debhelper-related information for Debian releases.

/// Retrieve the maximum supported debhelper compat version for a release.
///
/// # Arguments
/// * `compat_release` - A release name (Debian or Ubuntu, currently)
///
/// # Returns
/// The debhelper compat version, or `None` if the release is not known.
pub fn maximum_debhelper_compat_version(compat_release: &str) -> Option<u8> {
    crate::key_package_versions::debhelper_versions
        .get(compat_release)
        .map(|v| {
            v.upstream_version
                .split('.')
                .next()
                .unwrap()
                .parse()
                .unwrap()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_release() {
        assert!(maximum_debhelper_compat_version("sid").is_some());
        assert!(maximum_debhelper_compat_version("trixie").is_some());
    }

    #[test]
    fn test_unknown_release() {
        assert_eq!(None, maximum_debhelper_compat_version("nonexistent"));
    }
}
