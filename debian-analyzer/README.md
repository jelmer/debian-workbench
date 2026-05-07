debian-analyzer
===============

This Rust crate provides utilities for analyzing the Debian project and
its infrastructure: information about Debian packages, key package
versions, transitions, salsa metadata, snapshot.debian.org access, WNPP
(work-needing and prospective packages) bug parsing, debhelper
compatibility data, and ben transition files.

It is intended as a building block for tooling that introspects or makes
changes to Debian packages. It is used as part of a larger toolchain
including the [Debian Codemods](https://salsa.debian.org/jelmer/debian-codemods)
project, and is the analysis counterpart to the
[debian-workbench](../debian-workbench) crate, which performs the
actual modifications.

## Modules

* `benfile` — parser for ben transition files
* `debhelper` — debhelper compatibility level information
* `key_package_versions` — versions of key Debian packages, used to
  decide compatibility
* `salsa` — helpers for working with metadata from salsa.debian.org
* `snapshot` — access to snapshot.debian.org
* `transition` — Debian release transition data
* `udd` (optional, behind the `udd` feature) — queries against the
  Ultimate Debian Database
* `wnpp` (optional, behind the `udd` feature) — queries for WNPP bugs

## Features

* `udd` — enables the `udd` and `wnpp` modules, pulling in `sqlx` for
  PostgreSQL access.

## License

GPL-2.0+
