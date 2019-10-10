#[macro_use]
pub(crate) mod macros {
    macro_rules! bad_value_or_err {
        ( $config:expr, $err:expr ) => {
            if $config.strict {
                return Err($err);
            } else {
                HoconValue::BadValue($err)
            }
        };
    }

    macro_rules! public_bad_value_or_err {
        ( $config:expr, $err:expr ) => {
            if $config.strict {
                return Err($err);
            } else {
                Hocon::BadValue($err)
            }
        };
    }
}

mod intermediate;
mod internal;
mod value;
pub(crate) use internal::*;
pub(crate) use value::*;
