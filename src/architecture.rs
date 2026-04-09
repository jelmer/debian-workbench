//! Debian architecture list from `dpkg-architecture`.
//!
//! Provides a sorted list of all known Debian architecture names
//! by running `dpkg-architecture -L`.

use std::io::BufRead;

/// Get a sorted list of all known Debian architecture names.
///
/// Runs `dpkg-architecture -L` and collects the output. Returns an
/// empty list if the command is not available or fails.
pub fn get_architectures() -> Vec<String> {
    let Ok(output) = std::process::Command::new("dpkg-architecture")
        .arg("-L")
        .output()
    else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let mut arches: Vec<String> = output
        .stdout
        .lines()
        .map_while(Result::ok)
        .filter(|l| !l.is_empty())
        .collect();
    arches.sort();
    arches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_architectures() {
        let arches = get_architectures();
        // dpkg-architecture should be available on any Debian-based system
        if arches.is_empty() {
            // Possibly not on a Debian system; just verify no panic
            return;
        }
        assert!(arches.contains(&"amd64".to_string()));
        assert!(arches.contains(&"arm64".to_string()));
        // Should be sorted
        let mut sorted = arches.clone();
        sorted.sort();
        assert_eq!(arches, sorted);
    }
}
