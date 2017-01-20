# A lite thread pool library written for Rust. 

## Usage
Cargo.toml

```toml
    [dependencies]
    poolite = "0.4.0"
```
or
```toml
    [dependencies]  
    poolite = { git = "https://github.com/biluohc/poolite",branch = "master", version = "0.4.0" }
```

## documentation  
* Visit [https://docs.rs/poolite/](https://docs.rs/poolite/)  
or 
* Run `cargo doc --open` after modified the toml file.

## ChangLog
* 2017-0120 0.4.0 add `daemon()` ,`num_cpus()` methods, and move documentation to [doc.rs](https://docs.rs/poolite/).
* 2017-0112 0.3.0 remove all `unwrap()` and add `load_limit(),is_empty(), tasks_len(), len(), wait_len(), strong_count()` methods.
* 2016-0102 0.2.1 use unstable `FnBox()` to support `FnOnce()`(Only support Nightly now,Stable or Beta should use 0.2.0).
* 2016-0101 0.2.0 add `min(),time_out(),name(),stack_size(),run()` methods.