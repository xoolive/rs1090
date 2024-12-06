use std::io::{BufRead, BufReader, Cursor, Read};

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use rayon::prelude::*;
use rs1090::prelude::*;

const FLIGHT_CSV: &str = include_str!("../data/long_flight.csv");

fn first_lines(n: usize) {
    for line in FLIGHT_CSV.lines().take(n) {
        let mut parts = line.split(',');
        let _ts = parts.next().unwrap().parse::<f64>().expect("not a float");
        let msg = parts.next().unwrap();
        let hex = &mut msg.to_string()[18..].to_string();
        let bytes = hex::decode(hex).unwrap();
        let (_, _msg) = Message::from_bytes((&bytes, 0)).unwrap();
    }
}

struct ChunkedLines<'a> {
    reader: BufReader<Cursor<&'a str>>,
    chunk_size: usize,
}

impl<'a> ChunkedLines<'a> {
    fn new(content: &'a str, chunk_size: usize) -> Self {
        let cursor = Cursor::new(content);
        let reader = BufReader::new(cursor);
        ChunkedLines { reader, chunk_size }
    }
}

impl Iterator for ChunkedLines<'_> {
    type Item = Vec<String>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut chunk = Vec::with_capacity(self.chunk_size);
        for line in self.reader.by_ref().lines().take(self.chunk_size) {
            match line {
                Ok(line) => chunk.push(line),
                Err(_) => return None,
            }
        }
        if chunk.is_empty() {
            None
        } else {
            Some(chunk)
        }
    }
}

fn long_flight() -> Vec<Vec<Message>> {
    let chunks: Vec<_> = ChunkedLines::new(FLIGHT_CSV, 1000).collect();

    chunks
        .par_iter()
        .map(|lines| {
            let mut res = Vec::with_capacity(1000);
            for line in lines {
                let mut parts = line.split(',');

                let _ts =
                    parts.next().unwrap().parse::<f64>().expect("not a float");
                let msg = parts.next().unwrap();
                let hex = &mut msg.to_string()[18..].to_string();
                let bytes = hex::decode(hex).unwrap();
                let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
                res.push(msg);
            }
            res
        })
        .collect()
}

fn bench_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("linear");
    let n = 1_000;
    group.throughput(Throughput::Elements(n));
    group.bench_function(format!("{} lines", n), |b| {
        b.iter(|| first_lines(n as usize))
    });
    group.finish();

    let mut group = c.benchmark_group("parallel");
    group.throughput(Throughput::Elements(172432)); // number of lines
    group.bench_function("rayon", |b| {
        b.iter(|| {
            let res = long_flight();
            let _ = res; // force the actual evaluation
        })
    });
    group.finish();
}

criterion_group!(benches, bench_file);
criterion_main!(benches);
