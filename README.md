debian-workbench
================

This repository is a Cargo workspace containing two Rust crates for
working with Debian source packages:

* [`debian-analyzer`](debian-analyzer/) — introspection of Debian
  packages and infrastructure: key package versions, transitions, salsa
  metadata, snapshot.debian.org, WNPP bugs, debhelper levels, ben
  transition files.
* [`debian-workbench`](debian-workbench/) — modification of Debian
  source packages: editing `debian/control`, `debian/changelog`,
  `debian/rules`, copyright files, quilt patches; VCS metadata;
  detection of changelog conventions; debcargo and vendoring helpers.
  Also ships the `detect-changelog-behaviour` and `deb-vcs-publish`
  binaries.

Together they form a building block for tooling that makes consistent,
automated changes to Debian packages. One of the key users is the
[Debian Codemods](https://salsa.debian.org/jelmer/debian-codemods)
project.

## Building

```sh
cargo build --workspace
```

## Maintenance

`make update` regenerates `debian-analyzer/key-package-versions.json`
from the Debian archive. The data is consumed at build time by
`debian-analyzer`.

## License

GPL-2.0+
