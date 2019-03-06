# HOCON.rs [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Build Status](https://travis-ci.org/mockersf/hocon.rs.svg?branch=master)](https://travis-ci.org/mockersf/hocon.rs) [![Coverage Status](https://coveralls.io/repos/github/mockersf/hocon.rs/badge.svg?branch=master)](https://coveralls.io/github/mockersf/hocon.rs?branch=master) [![Realease Doc](https://docs.rs/hocon/badge.svg)](https://docs.rs/hocon) [![Crate](https://img.shields.io/crates/v/hocon.svg)](https://crates.io/crates/hocon)

Parse HOCON configuration files in Rust

The API docs for the master branch are published [here](https://mockersf.github.io/hocon.rs/).

## Usage

```rust
let s = r#"{"a":5}"#;
let doc: Hocon = HoconLoader::from_str(s)?;

assert_eq!(doc["a"].as_i64().unwrap(), 5);
```

```rust
let s = r#"{"b":5, "b":10}"#;
let doc: Hocon = HoconLoader::from_str(s)?;

assert_eq!(doc["b"].as_i64().unwrap(), 10);
```

Serde support is enabled by default and can be used to deserialize HOCON documents to `struct`s. It can be disabled by disabling default features.

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Configuration {
    host: String,
    port: u8,
    auto_connect: bool,
}

let s = r#"{host: 127.0.0.1, port: 80, auto_connect: false}"#;

let conf: Configuration = hocon::serde::from_str(s)?;
```

## Status

https://github.com/lightbend/config/blob/master/HOCON.md

- [x] parsing JSON
- [x] comments
- [x] omit root braces
- [x] key-value separator
- [x] commas are optional if newline is present
- [x] whitespace
- [x] duplicate keys and object merging
- [x] unquoted strings
- [x] multi-line strings
- [x] value concatenation
- [x] object concatenation
- [x] array concatenation
- [x] path expressions
- [x] path as keys
- [x] substitutions
- [x] includes
- [x] conversion of numerically-indexed objects to arrays
- [x] allow URL for included files
- [x] duration unit format
- [x] period unit format
- [x] size unit format
