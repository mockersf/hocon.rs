# Changes

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
