//! Lintian data structures and utilities

/// The path to the Lintian data directory
pub const LINTIAN_DATA_PATH: &str = "/usr/share/lintian/data";

/// The path to the Lintian release dates file (old name)
pub const RELEASE_DATES_PATH_OLD: &str = "/usr/share/lintian/data/debian-policy/release-dates.json";

/// The path to the Lintian release dates file (new name)
pub const RELEASE_DATES_PATH_NEW: &str = "/usr/share/lintian/data/debian-policy/releases.json";

#[derive(Debug, Clone)]
/// A release of the Debian Policy
pub struct PolicyRelease {
    /// The version of the release
    pub version: StandardsVersion,
    /// When the release was published
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// List of bug numbers closed by this release
    pub closes: Vec<i32>,
    /// The epoch of the release
    pub epoch: Option<i32>,
    /// The author of the release
    pub author: Option<String>,
    /// The changes made in this release
    pub changes: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct Preamble {
    pub cargo: String,
    pub title: String,
}

// Internal struct for deserializing releases.json (new format with floats)
#[derive(Debug, Clone, serde::Deserialize)]
struct PolicyReleaseNewFormat {
    pub version: StandardsVersion,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub closes: Vec<f64>,
    pub epoch: Option<i32>,
    pub author: Option<String>,
    pub changes: Vec<String>,
}

// Internal struct for deserializing release-dates.json (old format with ints)
#[derive(Debug, Clone, serde::Deserialize)]
struct PolicyReleaseOldFormat {
    pub version: StandardsVersion,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub closes: Vec<i32>,
    pub epoch: Option<i32>,
    pub author: Option<String>,
    pub changes: Vec<String>,
}

impl From<PolicyReleaseNewFormat> for PolicyRelease {
    fn from(r: PolicyReleaseNewFormat) -> Self {
        PolicyRelease {
            version: r.version,
            timestamp: r.timestamp,
            closes: r.closes.into_iter().map(|c| c as i32).collect(),
            epoch: r.epoch,
            author: r.author,
            changes: r.changes,
        }
    }
}

impl From<PolicyReleaseOldFormat> for PolicyRelease {
    fn from(r: PolicyReleaseOldFormat) -> Self {
        PolicyRelease {
            version: r.version,
            timestamp: r.timestamp,
            closes: r.closes,
            epoch: r.epoch,
            author: r.author,
            changes: r.changes,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct PolicyReleasesNewFormat {
    pub preamble: Preamble,
    pub releases: Vec<PolicyReleaseNewFormat>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct PolicyReleasesOldFormat {
    pub preamble: Preamble,
    pub releases: Vec<PolicyReleaseOldFormat>,
}

#[derive(Debug, Clone)]
/// A version of the Debian Policy
pub struct StandardsVersion(Vec<i32>);

impl StandardsVersion {
    /// Create a new StandardsVersion from major, minor, and patch numbers
    pub fn new(major: i32, minor: i32, patch: i32) -> Self {
        Self(vec![major, minor, patch])
    }

    fn normalize(&self, n: usize) -> Self {
        let mut version = self.0.clone();
        version.resize(n, 0);
        Self(version)
    }
}

impl std::cmp::PartialEq for StandardsVersion {
    fn eq(&self, other: &Self) -> bool {
        // Normalize to the same length
        let n = std::cmp::max(self.0.len(), other.0.len());
        let self_normalized = self.normalize(n);
        let other_normalized = other.normalize(n);
        self_normalized.0 == other_normalized.0
    }
}

impl std::cmp::Ord for StandardsVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Normalize to the same length
        let n = std::cmp::max(self.0.len(), other.0.len());
        let self_normalized = self.normalize(n);
        let other_normalized = other.normalize(n);
        self_normalized.0.cmp(&other_normalized.0)
    }
}

impl std::cmp::PartialOrd for StandardsVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Eq for StandardsVersion {}

impl std::str::FromStr for StandardsVersion {
    type Err = core::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('.').map(|part| part.parse::<i32>());
        let mut version = Vec::new();
        for part in &mut parts {
            version.push(part?);
        }
        Ok(StandardsVersion(version))
    }
}

impl<'a> serde::Deserialize<'a> for StandardsVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for StandardsVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|part| part.to_string())
                .collect::<Vec<_>>()
                .join(".")
        )
    }
}

/// Returns an iterator over all known standards versions
pub fn iter_standards_versions() -> impl Iterator<Item = PolicyRelease> {
    iter_standards_versions_opt()
        .expect("Failed to read release dates from either releases.json or release-dates.json")
}

/// Returns an iterator over all known standards versions
/// Returns None if neither releases.json nor release-dates.json can be found
pub fn iter_standards_versions_opt() -> Option<impl Iterator<Item = PolicyRelease>> {
    // Try the new filename first, then fall back to the old one
    if let Ok(data) = std::fs::read(RELEASE_DATES_PATH_NEW) {
        // Try to parse as new format (releases.json)
        if let Ok(parsed) = serde_json::from_slice::<PolicyReleasesNewFormat>(&data) {
            return Some(
                parsed
                    .releases
                    .into_iter()
                    .map(|r| r.into())
                    .collect::<Vec<_>>()
                    .into_iter(),
            );
        }
    }

    // Fall back to old format (release-dates.json)
    if let Ok(data) = std::fs::read(RELEASE_DATES_PATH_OLD) {
        if let Ok(parsed) = serde_json::from_slice::<PolicyReleasesOldFormat>(&data) {
            return Some(
                parsed
                    .releases
                    .into_iter()
                    .map(|r| r.into())
                    .collect::<Vec<_>>()
                    .into_iter(),
            );
        }
    }

    None
}

/// Returns the latest standards version
pub fn latest_standards_version() -> StandardsVersion {
    iter_standards_versions()
        .next()
        .expect("No standards versions found")
        .version
}

/// Returns the latest standards version
/// Returns None if release data files are not available
pub fn latest_standards_version_opt() -> Option<StandardsVersion> {
    iter_standards_versions_opt()
        .and_then(|mut iter| iter.next())
        .map(|release| release.version)
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    #[test]
    fn test_standards_version() {
        let version: super::StandardsVersion = "4.2.0".parse().unwrap();
        assert_eq!(version.0, vec![4, 2, 0]);
        assert_eq!(version.to_string(), "4.2.0");
        assert_eq!(version, "4.2".parse().unwrap());
        assert_eq!(version, "4.2.0".parse().unwrap());
    }

    #[test]
    fn test_parse_releases() {
        let input = r###"{
   "preamble" : {
      "cargo" : "releases",
      "title" : "Debian Policy Releases"
   },
   "releases" : [
      {
         "author" : "Sean Whitton <spwhitton@spwhitton.name>",
         "changes" : [
            "",
            "debian-policy (4.7.0.0) unstable; urgency=medium",
            "",
            "  [ Sean Whitton ]",
            "  * Policy: Prefer native overriding mechanisms to diversions & alternatives",
            "    Wording: Luca Boccassi <bluca@debian.org>",
            "    Seconded: Sean Whitton <spwhitton@spwhitton.name>",
            "    Seconded: Russ Allbery <rra@debian.org>",
            "    Seconded: Holger Levsen <holger@layer-acht.org>",
            "    Closes: #1035733",
            "  * Policy: Improve alternative build dependency discussion",
            "    Wording: Russ Allbery <rra@debian.org>",
            "    Seconded: Wouter Verhelst <wouter@debian.org>",
            "    Seconded: Sean Whitton <spwhitton@spwhitton.name>",
            "    Closes: #968226",
            "  * Policy: No network access for required targets for contrib & non-free",
            "    Wording: Aurelien Jarno <aurel32@debian.org>",
            "    Seconded: Sam Hartman <hartmans@debian.org>",
            "    Seconded: Tobias Frost <tobi@debian.org>",
            "    Seconded: Holger Levsen <holger@layer-acht.org>",
            "    Closes: #1068192",
            "",
            "  [ Russ Allbery ]",
            "  * Policy: Add mention of the new non-free-firmware archive area",
            "    Wording: Gunnar Wolf <gwolf@gwolf.org>",
            "    Seconded: Holger Levsen <holger@layer-acht.org>",
            "    Seconded: Russ Allbery <rra@debian.org>",
            "    Closes: #1029211",
            "  * Policy: Source packages in main may build binary packages in contrib",
            "    Wording: Simon McVittie <smcv@debian.org>",
            "    Seconded: Holger Levsen <holger@layer-acht.org>",
            "    Seconded: Russ Allbery <rra@debian.org>",
            "    Closes: #994008",
            "  * Policy: Allow hard links in source packages",
            "    Wording: Russ Allbery <rra@debian.org>",
            "    Seconded: Helmut Grohne <helmut@subdivi.de>",
            "    Seconded: Guillem Jover <guillem@debian.org>",
            "    Closes: #970234",
            "  * Policy: Binary and Description fields may be absent in .changes",
            "    Wording: Russ Allbery <rra@debian.org>",
            "    Seconded: Sam Hartman <hartmans@debian.org>",
            "    Seconded: Guillem Jover <guillem@debian.org>",
            "    Closes: #963524",
            "  * Policy: systemd units are required to start and stop system services",
            "    Wording: Luca Boccassi <bluca@debian.org>",
            "    Wording: Russ Allbery <rra@debian.org>",
            "    Seconded: Luca Boccassi <bluca@debian.org>",
            "    Seconded: Sam Hartman <hartmans@debian.org>",
            "    Closes: #1039102"
         ],
         "closes" : [
            963524,
            968226,
            970234,
            994008,
            1029211,
            1035733,
            1039102,
            1068192
         ],
         "epoch" : 1712466535,
         "timestamp" : "2024-04-07T05:08:55Z",
         "version" : "4.7.0.0"
      }
   ]
}"###;
        let data: super::PolicyReleasesOldFormat = serde_json::from_str(input).unwrap();
        assert_eq!(data.releases.len(), 1);
    }

    #[test]
    fn test_iter_standards_versions_opt() {
        // This test verifies that we can read the policy release data
        // In test environments, the files may not exist
        let Some(iter) = super::iter_standards_versions_opt() else {
            // Skip test if no files are available
            return;
        };

        let versions: Vec<_> = iter.collect();

        // Should have at least one version
        assert!(!versions.is_empty());

        // The latest version should be first
        let latest = &versions[0];

        // Verify the version has a proper format
        assert!(!latest.version.to_string().is_empty());
        assert!(latest.version.to_string().contains('.'));

        // Verify other fields are populated
        assert!(latest.timestamp.year() >= 2020);
        assert!(!latest.changes.is_empty());
    }

    #[test]
    fn test_latest_standards_version_opt() {
        // Test that we can get the latest standards version
        let Some(latest) = super::latest_standards_version_opt() else {
            // Skip test if no files are available
            return;
        };

        // Should have a valid version string
        let version_str = latest.to_string();
        assert!(!version_str.is_empty());
        assert!(version_str.contains('.'));

        // Should be at least 4.0.0 (Debian policy versions)
        assert!(latest >= "4.0.0".parse::<super::StandardsVersion>().unwrap());
    }
}
