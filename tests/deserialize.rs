use serde::Deserialize;

use hocon::HoconLoader;

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
        a: String,
    }
    #[derive(Deserialize, Debug)]
    struct Inner {
        ii: InnerInner,
    }
    #[derive(Deserialize, Debug)]
    struct Test {
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
