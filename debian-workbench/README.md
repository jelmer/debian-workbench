debian-workbench
================

This Rust crate provides a library and a small set of binaries for
manipulating Debian source packages: editing `debian/control`,
`debian/changelog`, `debian/rules`, copyright files and quilt patches;
detecting `gbp dch` and changelog conventions; managing VCS metadata,
maintainer scripts, debhelper levels, debcargo packaging, vendoring,
upstream version comparison and Debian release information.

It is intended as a building block for tooling that automates changes
to Debian packages, and is the modification counterpart to the
[debian-analyzer](../debian-analyzer) crate, which provides
introspection.

## Binaries

Both binaries require the `cli` feature:

* `detect-changelog-behaviour` — inspects a packaging branch and
  detects the changelog editing behaviour in use (e.g. `gbp dch`).
* `deb-vcs-publish` — publishes packaging changes to a VCS.

```sh
cargo install --features cli debian-workbench
```

## Library highlights

* `apply_or_revert` — run a closure inside a working tree and roll the
  tree back if the closure fails or makes no changes.
* `abstract_control`, `control`, `relations` — read and edit
  `debian/control`.
* `changelog`, `detect_gbp_dch` — manipulate `debian/changelog` and
  detect the maintenance style.
* `patches` — work with quilt patches.
* `publish`, `vcs` — VCS metadata and publishing helpers.
* `debcargo`, `vendor`, `versions`, `release_info` — utilities around
  packaging Rust crates and tracking upstream/Debian versions.
* `editor` — generic in-place editor that handles atomic writes and
  formatting preservation.

## Features

* `cli` — builds the binaries (pulls in `clap` and `env_logger`).
* `merge3` (default) — enables three-way merging support.
* `svp` — integrates with the Silver-Platter `svp-client`.
* `debian` — pulls in `debian-analyzer` for combined analysis +
  modification workflows.

## License

GPL-2.0+
