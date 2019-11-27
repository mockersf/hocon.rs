# Changes

## 0.3.1

* fixed various cases around array of object concatenation and substitutions
[#13](https://github.com/mockersf/hocon.rs/issues/13)
* fixed deserialization of tuple, enum and unit types 
* fixed deserialization of missing fields using default [#11](https://github.com/mockersf/hocon.rs/issues/11)
* fixed ignoring the end of a file if it was invalid Hocon [#15](https://github.com/mockersf/hocon.rs/pull/15)
* can take a Path to load a file [#10](https://github.com/mockersf/hocon.rs/issues/10)
* can resolve to a struct only part of the parsed Hocon document to help with complex cases
[#9](https://github.com/mockersf/hocon.rs/pull/9)
* improved error messages [#14](https://github.com/mockersf/hocon.rs/pull/14)

## 0.3.0

* error management
* introducing strict mode where errors will be returned instead of included in the document as `Hocon::BadValue`

## 0.2.0

* better coverage of HOCON specs
* `HoconLoader` object to allow for parsing configuration
* read system variable for configuration (can be disabled)

## 0.1.1

* support `null` value in hocon
* added feature `serde-support` to allow deserializing HOCON documents to a struct, enabled by default
* fixed crashes on some special cases
