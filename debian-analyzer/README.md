debian-analyzer
===============

This rust crate provides information about the Debian distribution —
suites, releases, key package versions, and related metadata.

It is designed to be used as part of a larger toolchain for making
changes to Debian packages. One of the key users of it is the
[Debian Codemods](https://salsa.debian.org/jelmer/debian-codemods)
project, which uses it to make changes to Debian packages in a
consistent and automated way.
