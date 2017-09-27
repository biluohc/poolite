[![Build status](https://travis-ci.org/biluohc/poolite.svg?branch=master)](https://github.com/biluohc/poolite)
[![Latest version](https://img.shields.io/crates/v/poolite.svg)](https://crates.io/crates/poolite)
[![All downloads](https://img.shields.io/crates/d/poolite.svg)](https://crates.io/crates/poolite)
[![Downloads of latest version](https://img.shields.io/crates/dv/poolite.svg)](https://crates.io/crates/poolite)
[![Documentation](https://docs.rs/poolite/badge.svg)](https://docs.rs/poolite)

## [poolite](https://github.com/biluohc/poolite)

A lite threadpool library written for Rust.

### Usage

On Cargo.toml:

```toml
 [dependencies]
 poolite = "0.6.4"
```

### Documentation
* Visit [Docs.rs](https://docs.rs/poolite/)

or

* Run `cargo doc --open` after modified the toml file.

### [Examples](https://github.com/biluohc/poolite/blob/master/examples/)
* [without return values](https://github.com/biluohc/poolite/blob/master/examples/without.rs)

* [return values by `Arc<Mutex<T>>`](https://github.com/biluohc/poolite/blob/master/examples/arc_mutex.rs)

* [return values by `channel`](https://github.com/biluohc/poolite/blob/master/examples/channel.rs)

* [about `IntoIOResult` and `IntoPool`](https://github.com/biluohc/poolite/blob/master/examples/into.rs)
