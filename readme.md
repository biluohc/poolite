# A lite thread pool library written for Rust. 

## Usage
Cargo.toml

```toml
    [dependencies]
    poolite = "0.5.1"
```
or
```toml
    [dependencies]  
    poolite = { git = "https://github.com/biluohc/poolite",branch = "master", version = "0.5.1" }
```

## Documentation  
* Visit [https://docs.rs/poolite/](https://docs.rs/poolite/)  
or 
* Run `cargo doc --open` after modified the toml file.

## ChangLog
* 2017-0129 0.5.1 add `add_threads()` method,complete `daemon()` method.

* 2017-0128 0.5.0 update API, `run(self)->self` to `run(self)->Result<Self, PoolError>`, add `IntoPool` and `IntoIOResult`(trait).

* 2017-0123 0.4.4 add `max()` method to set maximum number of threads.

* 2017-0121 0.4.3 Fix the document about `load_limit()`,the bug about block already fixed by last commit.

* 2017-0121 0.4.2 Fix a bug `attempt to divide by zero` and complete tests.

* 2017-0121 0.4.1 Remove constants's `pub`,modified `daemon()`(bool->Option<64>),change the default value of load_limit and reorder the document.

* 2017-0120 0.4.0 add `daemon()` ,`num_cpus()` methods, and move documentation to [doc.rs](https://docs.rs/poolite/).

* 2017-0112 0.3.0 remove all `unwrap()` and add `load_limit(),is_empty(), tasks_len(), len(), wait_len(), strong_count()` methods.

* 2016-0102 0.2.1 use unstable `FnBox()` to support `FnOnce()`(Only support Nightly now,Stable or Beta should use 0.2.0).

* 2016-0101 0.2.0 add `min(),time_out(),name(),stack_size(),run()` methods.