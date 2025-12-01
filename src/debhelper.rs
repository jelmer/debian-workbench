//! Debhelper utilities.
use debian_control::lossless::relations::Relations;
use debversion::Version;
use std::path::Path;

/// Parse the debhelper compat level from a string.
fn parse_debhelper_compat(s: &str) -> Option<u8> {
    s.split_once('#').map_or(s, |s| s.0).trim().parse().ok()
}

/// Read a debian/compat file.
///
/// # Arguments
/// * `path` - The path to the debian/compat file.
pub fn read_debhelper_compat_file(path: &Path) -> Result<Option<u8>, std::io::Error> {
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(parse_debhelper_compat(&content)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Retrieve the debhelper compat level from a debian/control file.
///
/// # Arguments
/// * `control` - The debian/control file.
///
/// # Returns
/// The debhelper compat level.
pub fn get_debhelper_compat_level_from_control(control: &debian_control::Control) -> Option<u8> {
    let source = control.source()?;

    if let Some(dh_compat) = source.as_deb822().get("X-DH-Compat") {
        return parse_debhelper_compat(dh_compat.as_str());
    }

    let build_depends = source.build_depends()?;

    let rels = build_depends
        .entries()
        .flat_map(|entry| entry.relations().collect::<Vec<_>>())
        .find(|r| r.name() == "debhelper-compat");

    rels.and_then(|r| r.version().and_then(|v| v.1.to_string().parse().ok()))
}

/// Retrieve the debhelper compat level from a debian/compat file or debian/control file.
///
/// # Arguments
/// * `path` - The path to the debian/ directory.
///
/// # Returns
/// The debhelper compat level.
pub fn get_debhelper_compat_level(path: &Path) -> Result<Option<u8>, std::io::Error> {
    match read_debhelper_compat_file(&path.join("debian/compat")) {
        Ok(Some(level)) => {
            return Ok(Some(level));
        }
        Err(e) => {
            return Err(e);
        }
        Ok(None) => {}
    }

    let p = path.join("debian/control");

    match std::fs::File::open(p) {
        Ok(f) => {
            let control = debian_control::Control::read_relaxed(f).unwrap().0;
            Ok(get_debhelper_compat_level_from_control(&control))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Retrieve the maximum supported debhelper compat version fior a release.
///
/// # Arguments
/// * `compat_release` - A release name (Debian or Ubuntu, currently)
///
/// # Returns
/// The debhelper compat version
pub fn maximum_debhelper_compat_version(compat_release: &str) -> u8 {
    crate::release_info::debhelper_versions
        .get(compat_release)
        .map(|v| {
            v.upstream_version
                .split('.')
                .next()
                .unwrap()
                .parse()
                .unwrap()
        })
        .unwrap_or_else(lowest_non_deprecated_compat_level)
}

/// Ask dh_assistant for the supported compat levels.
///
/// Cache the result.
fn get_lintian_compat_levels() -> &'static SupportedCompatLevels {
    lazy_static::lazy_static! {
        static ref LINTIAN_COMPAT_LEVELS: SupportedCompatLevels = {
            // TODO(jelmer): ideally we should be getting these numbers from the compat-release
            // dh_assistant, rather than what's on the system
            let output = std::process::Command::new("dh_assistant")
                .arg("supported-compat-levels")
                .output()
                .expect("failed to run dh_assistant")
                .stdout;
            serde_json::from_slice(&output).expect("failed to parse dh_assistant output")
        };
    };
    &LINTIAN_COMPAT_LEVELS
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct SupportedCompatLevels {
    #[serde(rename = "HIGHEST_STABLE_COMPAT_LEVEL")]
    highest_stable_compat_level: u8,
    #[serde(rename = "LOWEST_NON_DEPRECATED_COMPAT_LEVEL")]
    lowest_non_deprecated_compat_level: u8,
    #[serde(rename = "LOWEST_VIRTUAL_DEBHELPER_COMPAT_LEVEL")]
    lowest_virtual_debhelper_compat_level: u8,
    #[serde(rename = "MAX_COMPAT_LEVEL")]
    max_compat_level: u8,
    #[serde(rename = "MIN_COMPAT_LEVEL")]
    min_compat_level: u8,
    #[serde(rename = "MIN_COMPAT_LEVEL_NOT_SCHEDULED_FOR_REMOVAL")]
    min_compat_level_not_scheduled_for_removal: u8,
}

/// Find the lowest non-deprecated debhelper compat level.
pub fn lowest_non_deprecated_compat_level() -> u8 {
    get_lintian_compat_levels().lowest_non_deprecated_compat_level
}

/// Find the highest stable debhelper compat level.
pub fn highest_stable_compat_level() -> u8 {
    get_lintian_compat_levels().highest_stable_compat_level
}

/// Error type for ensure_minimum_debhelper_version
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnsureDebhelperError {
    /// debhelper or debhelper-compat found in Build-Depends-Indep or Build-Depends-Arch
    DebhelperInWrongField(String),
    /// Complex rule for debhelper-compat (multiple alternatives or non-equal version)
    ComplexDebhelperCompatRule,
    /// debhelper-compat without version constraint
    DebhelperCompatWithoutVersion,
}

impl std::fmt::Display for EnsureDebhelperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnsureDebhelperError::DebhelperInWrongField(field) => {
                write!(f, "debhelper in {}", field)
            }
            EnsureDebhelperError::ComplexDebhelperCompatRule => {
                write!(f, "Complex rule for debhelper-compat, aborting")
            }
            EnsureDebhelperError::DebhelperCompatWithoutVersion => {
                write!(f, "debhelper-compat without version, aborting")
            }
        }
    }
}

impl std::error::Error for EnsureDebhelperError {}

/// Ensure that the package is at least using a specific version of debhelper.
///
/// This is a dedicated helper, since debhelper can now also be pulled in
/// with a debhelper-compat dependency.
///
/// # Arguments
/// * `source` - The source paragraph from debian/control
/// * `minimum_version` - The minimum version required
///
/// # Returns
/// Ok(true) if the Build-Depends field was modified,
/// Ok(false) if no change was needed,
/// Err(EnsureDebhelperError) if there was an error
///
/// # Examples
/// ```rust
/// use debian_analyzer::debhelper::ensure_minimum_debhelper_version;
///
/// let text = "Source: foo\nBuild-Depends: debhelper (>= 10)\n";
/// let mut control = debian_control::Control::read_relaxed(text.as_bytes()).unwrap().0;
/// let mut source = control.source().unwrap();
/// let changed = ensure_minimum_debhelper_version(&mut source, &"11".parse().unwrap()).unwrap();
/// assert!(changed);
/// assert_eq!(source.build_depends().unwrap().to_string(), "debhelper (>= 11)");
/// ```
pub fn ensure_minimum_debhelper_version(
    source: &mut debian_control::lossless::Source,
    minimum_version: &Version,
) -> Result<bool, EnsureDebhelperError> {
    // Check that debhelper is not in Build-Depends-Indep or Build-Depends-Arch
    for (field_name, rels_opt) in [
        ("Build-Depends-Arch", source.build_depends_arch()),
        ("Build-Depends-Indep", source.build_depends_indep()),
    ] {
        let Some(rels) = rels_opt else {
            continue;
        };

        for entry in rels.entries() {
            for rel in entry.relations() {
                if rel.name() == "debhelper-compat" || rel.name() == "debhelper" {
                    return Err(EnsureDebhelperError::DebhelperInWrongField(
                        field_name.to_string(),
                    ));
                }
            }
        }
    }

    let mut rels = source.build_depends().unwrap_or_else(Relations::new);

    // Check if debhelper-compat is present
    for entry in rels.entries() {
        let has_debhelper_compat = entry
            .relations()
            .any(|rel| rel.name() == "debhelper-compat");

        if !has_debhelper_compat {
            continue;
        }

        if entry.relations().count() > 1 {
            return Err(EnsureDebhelperError::ComplexDebhelperCompatRule);
        }

        let rel = entry.relations().next().unwrap();
        let Some((constraint, version)) = rel.version() else {
            return Err(EnsureDebhelperError::DebhelperCompatWithoutVersion);
        };

        if constraint != debian_control::relations::VersionConstraint::Equal {
            return Err(EnsureDebhelperError::ComplexDebhelperCompatRule);
        }

        if &version >= minimum_version {
            return Ok(false);
        }
    }

    // Update or add debhelper dependency
    let changed = crate::relations::ensure_minimum_version(&mut rels, "debhelper", minimum_version);

    if changed {
        source.set_build_depends(&rels);
    }

    Ok(changed)
}

/// Get the debhelper sequences from Build-Depends.
///
/// Extracts all dh-sequence-* packages from the Build-Depends field.
///
/// # Arguments
/// * `source` - The source paragraph from debian/control
///
/// # Returns
/// An iterator over sequence names (without the "dh-sequence-" prefix)
///
/// # Examples
/// ```rust
/// use debian_analyzer::debhelper::get_sequences;
///
/// let text = "Source: foo\nBuild-Depends: dh-sequence-python3, dh-sequence-nodejs\n";
/// let control = debian_control::Control::read_relaxed(text.as_bytes()).unwrap().0;
/// let source = control.source().unwrap();
/// let sequences: Vec<String> = get_sequences(&source).collect();
/// assert_eq!(sequences, vec!["python3", "nodejs"]);
/// ```
pub fn get_sequences(source: &debian_control::lossless::Source) -> impl Iterator<Item = String> {
    let build_depends = source.build_depends().unwrap_or_else(Relations::new);

    build_depends
        .entries()
        .flat_map(|entry| entry.relations().collect::<Vec<_>>())
        .filter_map(|rel| {
            let name = rel.name();
            if name.starts_with("dh-sequence-") {
                Some(name[12..].to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .into_iter()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_debhelper_compat() {
        assert_eq!(super::parse_debhelper_compat("9"), Some(9));
        assert_eq!(super::parse_debhelper_compat("9 # comment"), Some(9));
        assert_eq!(
            super::parse_debhelper_compat("9 # comment # comment"),
            Some(9)
        );
        assert_eq!(super::parse_debhelper_compat(""), None);
        assert_eq!(super::parse_debhelper_compat(" # comment"), None);
    }

    #[test]
    fn test_get_debhelper_compat_level_from_control() {
        let text = "Source: foo
Build-Depends: debhelper-compat (= 9)

Package: foo
Architecture: any
";

        let control = debian_control::Control::read_relaxed(&mut text.as_bytes())
            .unwrap()
            .0;

        assert_eq!(
            super::get_debhelper_compat_level_from_control(&control),
            Some(9)
        );
    }

    #[test]
    fn test_get_debhelper_compat_level_from_control_x_dh_compat() {
        let text = "Source: foo
X-DH-Compat: 9
Build-Depends: debhelper
";

        let control = debian_control::Control::read_relaxed(&mut text.as_bytes())
            .unwrap()
            .0;

        assert_eq!(
            super::get_debhelper_compat_level_from_control(&control),
            Some(9)
        );
    }

    mod ensure_minimum_debhelper_version_tests {
        use super::*;

        #[test]
        fn test_already() {
            let text = "Source: foo\nBuild-Depends: debhelper (>= 10)\n";
            let mut control = debian_control::Control::read_relaxed(text.as_bytes())
                .unwrap()
                .0;
            let mut source = control.source().unwrap();

            assert!(
                !ensure_minimum_debhelper_version(&mut source, &"10".parse().unwrap()).unwrap()
            );
            assert_eq!(
                source.build_depends().unwrap().to_string(),
                "debhelper (>= 10)"
            );

            assert!(!ensure_minimum_debhelper_version(&mut source, &"9".parse().unwrap()).unwrap());
            assert_eq!(
                source.build_depends().unwrap().to_string(),
                "debhelper (>= 10)"
            );
        }

        #[test]
        fn test_already_compat() {
            let text = "Source: foo\nBuild-Depends: debhelper-compat (= 10)\n";
            let mut control = debian_control::Control::read_relaxed(text.as_bytes())
                .unwrap()
                .0;
            let mut source = control.source().unwrap();

            assert!(
                !ensure_minimum_debhelper_version(&mut source, &"10".parse().unwrap()).unwrap()
            );
            assert_eq!(
                source.build_depends().unwrap().to_string(),
                "debhelper-compat (= 10)"
            );

            assert!(!ensure_minimum_debhelper_version(&mut source, &"9".parse().unwrap()).unwrap());
            assert_eq!(
                source.build_depends().unwrap().to_string(),
                "debhelper-compat (= 10)"
            );
        }

        #[test]
        fn test_bump() {
            let text = "Source: foo\nBuild-Depends: debhelper (>= 10)\n";
            let mut control = debian_control::Control::read_relaxed(text.as_bytes())
                .unwrap()
                .0;
            let mut source = control.source().unwrap();

            assert!(ensure_minimum_debhelper_version(&mut source, &"11".parse().unwrap()).unwrap());
            assert_eq!(
                source.build_depends().unwrap().to_string(),
                "debhelper (>= 11)"
            );
        }

        #[test]
        fn test_bump_compat() {
            let text = "Source: foo\nBuild-Depends: debhelper-compat (= 10)\n";
            let mut control = debian_control::Control::read_relaxed(text.as_bytes())
                .unwrap()
                .0;
            let mut source = control.source().unwrap();

            assert!(ensure_minimum_debhelper_version(&mut source, &"11".parse().unwrap()).unwrap());
            assert_eq!(
                source.build_depends().unwrap().to_string(),
                "debhelper (>= 11), debhelper-compat (= 10)"
            );

            assert!(
                ensure_minimum_debhelper_version(&mut source, &"11.1".parse().unwrap()).unwrap()
            );
            assert_eq!(
                source.build_depends().unwrap().to_string(),
                "debhelper (>= 11.1), debhelper-compat (= 10)"
            );
        }

        #[test]
        fn test_not_set() {
            let text = "Source: foo\n";
            let mut control = debian_control::Control::read_relaxed(text.as_bytes())
                .unwrap()
                .0;
            let mut source = control.source().unwrap();

            assert!(ensure_minimum_debhelper_version(&mut source, &"10".parse().unwrap()).unwrap());
            assert_eq!(
                source.build_depends().unwrap().to_string(),
                "debhelper (>= 10)"
            );
        }

        #[test]
        fn test_in_indep() {
            let text = "Source: foo\nBuild-Depends-Indep: debhelper (>= 9)\n";
            let mut control = debian_control::Control::read_relaxed(text.as_bytes())
                .unwrap()
                .0;
            let mut source = control.source().unwrap();

            let result = ensure_minimum_debhelper_version(&mut source, &"10".parse().unwrap());
            assert!(result.is_err());
            assert_eq!(
                result.unwrap_err(),
                EnsureDebhelperError::DebhelperInWrongField("Build-Depends-Indep".to_string())
            );
        }
    }

    mod get_sequences_tests {
        use super::*;

        #[test]
        fn test_no_sequences() {
            let text = "Source: foo\nBuild-Depends: debhelper (>= 10)\n";
            let control = debian_control::Control::read_relaxed(text.as_bytes())
                .unwrap()
                .0;
            let source = control.source().unwrap();

            let sequences: Vec<String> = get_sequences(&source).collect();
            assert_eq!(sequences, Vec::<String>::new());
        }

        #[test]
        fn test_single_sequence() {
            let text = "Source: foo\nBuild-Depends: dh-sequence-python3, debhelper (>= 10)\n";
            let control = debian_control::Control::read_relaxed(text.as_bytes())
                .unwrap()
                .0;
            let source = control.source().unwrap();

            let sequences: Vec<String> = get_sequences(&source).collect();
            assert_eq!(sequences, vec!["python3"]);
        }

        #[test]
        fn test_multiple_sequences() {
            let text = "Source: foo\nBuild-Depends: dh-sequence-python3, dh-sequence-nodejs, debhelper (>= 10)\n";
            let control = debian_control::Control::read_relaxed(text.as_bytes())
                .unwrap()
                .0;
            let source = control.source().unwrap();

            let sequences: Vec<String> = get_sequences(&source).collect();
            assert_eq!(sequences, vec!["python3", "nodejs"]);
        }

        #[test]
        fn test_no_build_depends() {
            let text = "Source: foo\n";
            let control = debian_control::Control::read_relaxed(text.as_bytes())
                .unwrap()
                .0;
            let source = control.source().unwrap();

            let sequences: Vec<String> = get_sequences(&source).collect();
            assert_eq!(sequences, Vec::<String>::new());
        }
    }
}
