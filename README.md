# HOCON.rs [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Build Status](https://travis-ci.org/mockersf/hocon.rs.svg?branch=master)](https://travis-ci.org/mockersf/hocon.rs) [![Coverage Status](https://coveralls.io/repos/github/mockersf/hocon.rs/badge.svg?branch=master)](https://coveralls.io/github/mockersf/hocon.rs?branch=master) [![Realease Doc](https://docs.rs/hocon/badge.svg)](https://docs.rs/hocon) [![Crate](https://img.shields.io/crates/v/hocon.svg)](https://crates.io/crates/hocon)

The API docs for the master branch are published [here](https://mockersf.github.io/hocon.rs/).

Parse HOCON configuration files in Rust following the
[HOCON Specifications](https://github.com/lightbend/config/blob/master/HOCON.md).

This implementation goal is to be as permissive as possible, returning a valid document
with all errors wrapped in `Hocon::BadValue`. `strict` mode can be enabled to return the
first `Error` encountered instead.

## Examples

### Parsing a string to a struct using serde

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Configuration {
    host: String,
    port: u8,
    auto_connect: bool,
}

fn main() -> Result<(), Error> {
    let s = r#"{
        host: 127.0.0.1
        port: 80
        auto_connect: false
    }"#;

    let conf: Configuration = hocon::de::from_str(s)?;

    Ok(())
}
```

### Reading from a string and getting value directly

```rust
use hocon::HoconLoader;

fn main() -> Result<(), Error> {
    let s = r#"{ a: 7 }"#;

    let doc = HoconLoader::new()
        .load_str(s)?
        .hocon()?;

    let a = doc["a"].as_i64();
    assert_eq!(a, Some(7));

    Ok(())
}
```

### Deserializing to a struct using `serde`

```rust
use serde::Deserialize;

use hocon::HoconLoader;

#[derive(Deserialize)]
struct Configuration {
    host: String,
    port: u8,
    auto_connect: bool,
}

fn main() -> Result<(), Error> {
    let s = r#"{
        host: 127.0.0.1
        port: 80
        auto_connect: false
    }"#;

    let conf: Configuration = HoconLoader::new()
        .load_str(s)?
        .resolve()?;

    Ok(())
}
```

### Reading from a file

```rust
use hocon::HoconLoader;

fn main() -> Result<(), Error> {
    let doc = HoconLoader::new()
        .load_file("tests/data/basic.conf")?
        .hocon()?;

    let a = doc["a"].as_i64();
    assert_eq!(a, Some(5));

    Ok(())
}
```

### Reading from several documents

```rust
use hocon::HoconLoader;

fn main() -> Result<(), Error> {
    let s = r#"{
        a: will be changed
        unchanged: original value
    }"#;

    let doc = HoconLoader::new()
        .load_str(s)?
        .load_file("tests/data/basic.conf")?
        .hocon()?;

    let a = doc["a"].as_i64();
    assert_eq!(a, Some(5));
    let unchanged = doc["unchanged"].as_string();
    assert_eq!(unchanged, Some(String::from("original value")));

    Ok(())
}
```

## Features

All features are enabled by default. They can be disabled to reduce dependencies.

### `url-support`

This feature enable fetching URLs in includes  with `include url("http://mydomain.com/myfile.conf")` (see
[spec](https://github.com/lightbend/config/blob/master/HOCON.md#include-syntax)). If disabled,
includes will only load local files specified with `include "path/to/file.conf"` or
`include file("path/to/file.conf")`.

### `serde-support`

This feature enable deserializing to a `struct` implementing `Deserialize` using `serde`

```rust
use serde::Deserialize;

use hocon::HoconLoader;

#[derive(Deserialize)]
struct Configuration {
    host: String,
    port: u8,
    auto_connect: bool,
}

# fn main() -> Result<(), Error> {
let s = r#"{host: 127.0.0.1, port: 80, auto_connect: false}"#;

# #[cfg(feature = "serde-support")]
let conf: Configuration = HoconLoader::new().load_str(s)?.resolve()?;
# Ok(())
# }
```

## Spec Coverage

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
