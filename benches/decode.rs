use criterion::{criterion_group, criterion_main, Criterion};
use deku::DekuContainerRead;
use rs1090::decode::Message;

const FLIGHT_CSV: &str = include_str!("flight.csv");

fn long_flight() {
    for line in FLIGHT_CSV.lines() {
        let mut parts = line.split(',');

        let _ts = parts.next().unwrap().parse::<f64>().expect("not a float");
        let msg = parts.next().unwrap();
        let hex = &mut msg.to_string()[18..].to_string();
        let bytes = hex::decode(&hex).unwrap();
        let _frame = Message::from_bytes((&bytes, 0)).unwrap().1;
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("long_flight", |b| b.iter(long_flight));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
