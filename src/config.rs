//! Lintian-brush configuration file.
use crate::Certainty;
use breezyshim::tree::WorkingTree;
use configparser::ini::Ini;
use log::warn;

const SUPPORTED_KEYS: &[&str] = &[
    "compat-release",
    "minimum-certainty",
    "allow-reformatting",
    "update-changelog",
];

/// Configuration file name
pub const PACKAGE_CONFIG_FILENAME: &str = "debian/lintian-brush.conf";

/// Configuration file
pub struct Config {
    obj: Ini,
}

impl Config {
    /// Load configuration from a working tree
    pub fn from_workingtree(
        tree: &dyn WorkingTree,
        subpath: &std::path::Path,
    ) -> std::io::Result<Self> {
        let path = tree
            .abspath(&subpath.join(PACKAGE_CONFIG_FILENAME))
            .unwrap();
        Self::load_from_path(&path)
    }

    /// Load configuration from a path
    pub fn load_from_path(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let mut ini = Ini::new();
        let data = std::fs::read_to_string(path)?;
        ini.read(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        for (section, contents) in ini.get_map_ref() {
            if section != "default" {
                warn!(
                    "unknown section {} in {}, ignoring.",
                    section,
                    path.display()
                );
                continue;
            }
            for key in contents.keys() {
                if !SUPPORTED_KEYS.contains(&key.as_str()) {
                    warn!(
                        "unknown key {} in section {} in {}, ignoring.",
                        key,
                        section,
                        path.display()
                    );

                    continue;
                }
            }
        }

        Ok(Config { obj: ini })
    }

    /// Return the compatibility release.
    pub fn compat_release(&self) -> Option<String> {
        self.obj.get("default", "compat-release").and_then(|value| {
            let codename = crate::release_info::resolve_release_codename(&value, None);
            if codename.is_none() {
                warn!("unknown compat release {}, ignoring.", value);
            }
            codename
        })
    }

    /// Return whether reformatting is allowed.
    pub fn allow_reformatting(&self) -> Option<bool> {
        match self.obj.getbool("default", "allow-reformatting") {
            Ok(value) => value,
            Err(e) => {
                warn!("invalid allow-reformatting value {}, ignoring.", e);
                None
            }
        }
    }

    /// Return the minimum certainty level for changes to be applied.
    pub fn minimum_certainty(&self) -> Option<Certainty> {
        self.obj
            .get("default", "minimum-certainty")
            .and_then(|value| {
                value
                    .parse::<Certainty>()
                    .inspect_err(|_e| {
                        warn!("invalid minimum-certainty value {}, ignoring.", value);
                    })
                    .ok()
            })
    }

    /// Return whether the changelog should be updated.
    pub fn update_changelog(&self) -> Option<bool> {
        match self.obj.getbool("default", "update-changelog") {
            Ok(value) => value,
            Err(e) => {
                warn!("invalid update-changelog value {}, ignoring.", e);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compat_release() {
        let td = tempfile::tempdir().unwrap();
        std::fs::create_dir(td.path().join("debian")).unwrap();
        std::fs::write(
            td.path().join("debian/lintian-brush.conf"),
            "compat-release = testing\n",
        )
        .unwrap();
        let cfg = Config::load_from_path(&td.path().join("debian/lintian-brush.conf")).unwrap();

        let testing = crate::release_info::resolve_release_codename("testing", None);

        assert_eq!(cfg.compat_release(), testing);
    }

    #[test]
    fn test_minimum_certainty() {
        let td = tempfile::tempdir().unwrap();
        std::fs::create_dir(td.path().join("debian")).unwrap();
        std::fs::write(
            td.path().join("debian/lintian-brush.conf"),
            "minimum-certainty = possible\n",
        )
        .unwrap();
        let cfg = Config::load_from_path(&td.path().join("debian/lintian-brush.conf")).unwrap();

        assert_eq!(cfg.minimum_certainty(), Some(Certainty::Possible));
    }

    #[test]
    fn test_update_changelog() {
        let td = tempfile::tempdir().unwrap();
        std::fs::create_dir(td.path().join("debian")).unwrap();
        std::fs::write(
            td.path().join("debian/lintian-brush.conf"),
            "update-changelog = True\n",
        )
        .unwrap();
        let cfg = Config::load_from_path(&td.path().join("debian/lintian-brush.conf")).unwrap();

        assert_eq!(cfg.update_changelog(), Some(true));
    }

    #[test]
    fn test_unknown() {
        let td = tempfile::tempdir().unwrap();
        std::fs::create_dir(td.path().join("debian")).unwrap();
        std::fs::write(
            td.path().join("debian/lintian-brush.conf"),
            "unknown = dunno\n",
        )
        .unwrap();
        let cfg = Config::load_from_path(&td.path().join("debian/lintian-brush.conf")).unwrap();
        assert_eq!(cfg.compat_release(), None);
    }

    #[test]
    fn test_missing() {
        let td = tempfile::tempdir().unwrap();
        let path = td.path().join("debian/lintian-brush.conf");
        let cfg = Config::load_from_path(&path);
        assert!(cfg.is_err());
    }
}
