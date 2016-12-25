# A lite thread pool library written for Rust. 

## Usage
Cargo.toml

    [dependencies]  
    poolite = { git = "https://github.com/biluohc/poolite" }

## Explain 
* use poolite::pool::new() create a thread_pool.  
* poolite.Pool.spawn receive Box<FnMut() + Send>.  
* while leave scope,pool will drop automatically.  

## Example  
    extern crate poolite;  

    let pool = poolite::Pool::new();  
    pool.spawn(Box::new(move || test(32)));

