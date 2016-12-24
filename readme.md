# A lite thread pool library written for Rust. 

## Usage
Cargo.toml

    [dependencies]  
    poolite = { git = "https://github.com/biluohc/poolite" }

## Example
    extern crate poolite;

    let pool = poolite::Pool::new();
    pool.spawn(Box::new(move || test(i)));
