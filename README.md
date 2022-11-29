# rust-ovh

[![Build](https://github.com/MicroJoe/rust-ovh/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/MicroJoe/rust-ovh/actions/workflows/ci.yml)
[![Latest version](https://img.shields.io/crates/v/ovh.svg)](https://crates.io/crates/ovh)
[![Documentation](https://docs.rs/ovh/badge.svg)](https://docs.rs/ovh)
[![License](https://img.shields.io/crates/l/ovh.svg)](https://crates.io/crates/ovh)

Async client for the OVH API.

[Creating tokens for accessing the OVH API](https://api.ovh.com/createToken/index.cgi?GET=/*&PUT=/*&POST=/*&DELETE=/*)

## High-level usage

Some parts of the API are implemented using typed Rust structs
and functions.

## Low-level usage

For all of the other API parts not already covered by a high-level
implementation, the low-level API part can be used as a fallback.

## License

Licensed under [GNU Affero General Public License v3.0](LICENSE-AGPL-3.0).

**@ovh**: if you want to:

* relicense this crate to a less strict license; *and/or*
* reclaim the `crates.io/crates/ovh` namespace; *and/or*
* become the maintainer of this crate

Then please contact me.
