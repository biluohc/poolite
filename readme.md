# A lite thread pool library written for Rust. 

## Usage
Cargo.toml

```toml
    [dependencies]  
    poolite = { git = "https://github.com/biluohc/poolite",branch = "master", version = "0.1.3" }
```

## Explain 
* use `poolite::pool::new()` create a thread_pool.  
* `spawn()` receive `Box<FnMut() + Send>`.  
* while leave scope,pool will drop automatically.  

## Example  
```Rust
    extern crate poolite;  

    let pool = poolite::Pool::new();  
    pool.spawn(Box::new(move || test(32)));
```
