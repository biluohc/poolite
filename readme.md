# poolite

## [A lite threadpool library written for Rust.](https://github.com/biluohc/poolite)

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

* [about IntoIOResult and IntoPool](https://github.com/biluohc/poolite/blob/master/examples/into.rs)

License: MIT
