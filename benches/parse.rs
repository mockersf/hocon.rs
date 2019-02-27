#[macro_use]
extern crate criterion;

use criterion::Criterion;

fn parse(file_name: &str) -> () {
    hocon::HoconLoader::new()
        .no_system()
        .load_file(file_name)
        .unwrap()
        .hocon()
        .unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("parse test01.conf", |b| {
        b.iter(|| parse("benches/data/test01.conf"))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
