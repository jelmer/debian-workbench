//! Information about the Debian project and its infrastructure.
#![deny(missing_docs)]

pub mod benfile;
#[cfg(feature = "udd")]
pub mod debbugs;
pub mod debhelper;
pub mod key_package_versions;
#[cfg(feature = "udd")]
pub mod popcon;
#[cfg(feature = "udd")]
pub mod rdeps;
pub mod salsa;
pub mod snapshot;
pub mod transition;
#[cfg(feature = "udd")]
pub mod udd;
#[cfg(feature = "udd")]
pub mod vcswatch;
#[cfg(feature = "udd")]
pub mod wnpp;
