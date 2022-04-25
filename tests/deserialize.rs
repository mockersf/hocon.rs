#![cfg(feature = "serde-support")]

use serde::Deserialize;

use hocon::HoconLoader;

#[test]
fn deserialize_struct_simple_path() {
    #[derive(Deserialize, Debug)]
    struct Test {
        a: String,
    }

    let s = r#"{"a":"dndjf"}"#;
    let doc: Test = dbg!(hocon::de::from_str(s)).expect("during test");

    assert_eq!(doc.a, "dndjf");
}

#[test]
fn deserialize_struct() {
    #[derive(Deserialize, Debug)]
    struct Test {
        a: String,
    }

    let s = r#"{"a":"dndjf"}"#;
    let doc = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test: error loading string")
        .hocon()
        .expect("during test: error parsing to hocon")
        .resolve::<Test>()
        .expect("during test: error deserializing");

    assert_eq!(doc.a, "dndjf");
}

#[test]
fn deserialize_struct_missing_field() {
    #[derive(Deserialize, Debug)]
    struct Test {
        #[allow(dead_code)]
        a: String,
    }

    let s = r#"{"b":"dndjf"}"#;
    let doc = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test: error loading string")
        .hocon()
        .expect("during test: error parsing to hocon")
        .resolve::<Test>();

    assert_eq!(
        format!("{:?}", doc),
        r#"Err(Deserialization { message: ".: missing field `a`" })"#
    );
}

#[test]
fn deserialize_multilevel_struct() {
    #[derive(Deserialize, Debug)]
    struct Inner {
        a: String,
    }
    #[derive(Deserialize, Debug)]
    struct Test {
        i: Inner,
    }

    let s = r#"{"i":{"a":"dndjf"}}"#;
    let doc = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test: error loading string")
        .hocon()
        .expect("during test: error parsing to hocon")
        .resolve::<Test>()
        .expect("during test: error deserializing");

    assert_eq!(doc.i.a, "dndjf");
}

#[test]
fn deserialize_multilevel_struct_missing_field() {
    #[derive(Deserialize, Debug)]
    struct InnerInner {
        #[allow(dead_code)]
        a: String,
    }
    #[derive(Deserialize, Debug)]
    struct Inner {
        #[allow(dead_code)]
        ii: InnerInner,
    }
    #[derive(Deserialize, Debug)]
    struct Test {
        #[allow(dead_code)]
        i: Inner,
    }

    let s = r#"{"i":{"ii":{"b":"dndjf"}}}"#;
    let doc = dbg!(HoconLoader::new().load_str(dbg!(s)))
        .expect("during test: error loading string")
        .hocon()
        .expect("during test: error parsing to hocon")
        .resolve::<Test>();

    assert_eq!(
        format!("{:?}", doc),
        r#"Err(Deserialization { message: "i.ii: missing field `a`" })"#
    );
}

#[test]
fn deserialize_struct_duration_wrapper() {
    use hocon::de::wrappers::Serde;
    use std::time::Duration;

    #[derive(Deserialize, Debug)]
    struct Test {
        a: Serde<Duration>,
    }

    let s = r#"{"a":"1 second"}"#;

    let doc: Test = dbg!(hocon::de::from_str(s)).expect("during test");

    assert_eq!(*doc.a, std::time::Duration::from_secs(1));
}

#[test]
fn deserialize_struct_duration_with() {
    use hocon::de::wrappers::Serde;
    use std::time::Duration;
    #[derive(Deserialize, Debug)]
    struct Test {
        #[serde(deserialize_with = "Serde::<Duration>::with")]
        a: std::time::Duration,
    }

    let s = r#"{"a":"1 second"}"#;

    let doc: Test = dbg!(hocon::de::from_str(s)).expect("during test");

    assert_eq!(doc.a, std::time::Duration::from_secs(1));
}

#[test]
fn deserialize_filesize() {
    #[derive(Deserialize, Debug)]
    struct Test {
        data: u32,
    }
    let s = r#"{
            data: 32.5M
        }"#;

    let doc: Test = dbg!(hocon::de::from_str(s)).expect("during test");
    assert_eq!(doc.data, 34078720);
}

#[test]
fn deserialize_filesize_as_float() {
    #[derive(Deserialize, Debug)]
    struct Test {
        data: f32,
    }
    let s = r#"{
            data: 2.5M
        }"#;

    let doc: Test = dbg!(hocon::de::from_str(s)).expect("during test");
    assert_eq!(doc.data, 2621440.0);
}
