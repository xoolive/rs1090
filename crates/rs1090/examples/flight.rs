use rayon::prelude::*;
use rs1090::decode::cpr::{decode_positions, Position};
use rs1090::prelude::*;

use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::vec::Vec;

struct ChunkedLines {
    reader: BufReader<File>,
    chunk_size: usize,
}

impl ChunkedLines {
    fn new(file_name: &str, chunk_size: usize) -> io::Result<Self> {
        let file = File::open(file_name)?;
        let reader = BufReader::new(file);
        Ok(ChunkedLines { reader, chunk_size })
    }
}

impl Iterator for ChunkedLines {
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

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let file_name = args.get(1).expect("Filename required");
    let reference = Position {
        latitude: 43.7,
        longitude: 1.36,
    };
    let lines: Vec<_> = ChunkedLines::new(file_name, 1000)?.collect();

    let mut res: Vec<TimedMessage> = lines
        .par_iter()
        .map(|lines| {
            let mut res = Vec::with_capacity(1000);
            for line in lines {
                let mut parts = line.split(',');

                let timestamp =
                    parts.next().unwrap().parse::<f64>().expect("not a float");
                let msg = parts.next().unwrap();
                let hex = &mut msg.to_string()[18..].to_string();
                // Without the &hex, a copy is necessary for the hex.to_string()
                // in the TimedMessage below.
                #[allow(clippy::needless_borrows_for_generic_args)]
                let bytes = hex::decode(&hex).unwrap();
                let (_, msg) = Message::from_bytes((&bytes, 0)).unwrap();
                res.push(TimedMessage {
                    timestamp,
                    frame: bytes,
                    message: Some(msg),
                    metadata: vec![],
                    decode_time: None,
                });
            }
            res
        })
        .flat_map(|v| v)
        .collect();

    // println!("{} messages processed", res.len());

    decode_positions(&mut res, Some(reference), &None);

    println!("{}", serde_json::to_string(&res).unwrap());
    Ok(())
}
