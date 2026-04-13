debian-workbench
================

This rust crate provides higher-level utilities for working with Debian
packages in a version-control-aware workbench: editing changelogs,
control files, copyright files, watch files, quilt patches, and
related VCS operations via [breezyshim].

It builds on top of [debian-analyzer] and is used as a foundation for
automated Debian package modification tools.

[breezyshim]: https://crates.io/crates/breezyshim
[debian-analyzer]: ../debian-analyzer
