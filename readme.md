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
 poolite = "0.7.1"
```

### Documentation
* Visit [Docs.rs](https://docs.rs/poolite/)

or

* Run `cargo doc --open` after modified the toml file.

### Base usage
```rust
extern crate poolite;
use poolite::Pool;

fn main() {
    let pool = Pool::new().unwrap();
    for i in 0..38 {
        pool.push(move || test(i));
    }

    pool.join(); //wait for the pool
}

fn test(msg: i32) {
    println!("key: {}\tvalue: {}", msg, fib(msg));
}

fn fib(msg: i32) -> i32 {
    match msg {
        0...2 => 1,
        x => fib(x - 1) + fib(x - 2),
    }
}
```

### `Scoped` `Task`
```rust
extern crate poolite;
use poolite::Pool;

fn main() {
    let pool = Pool::new().unwrap();
    let mut array = (0..100usize).into_iter().map(|i| (i, 0)).collect::<Vec<_>>();

    // scoped method will waiting scoped's task running finish.
    pool.scoped(|scope| for i in array.iter_mut() {
        // have to move
        scope.push(move|| i.1 = i.0*i.0);
    });

    for (i, j) in array {
        println!("key: {}\tvalue: {}", i, j);
    }
}
```

### [More Examples..](https://github.com/biluohc/poolite/blob/master/examples/)
