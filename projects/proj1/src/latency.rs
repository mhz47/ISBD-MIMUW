use std::env;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use rand::{rng, Rng, RngExt, SeedableRng};
use rand::rngs::StdRng;

mod hash;
use hash::sum_block_hash;

pub struct RandomOrderIterator {
    elements: Vec<u64>,
    rng: Box<dyn Rng>
}

impl RandomOrderIterator {
    pub fn with_seed(size: u64, seed: u64) -> Self {
        Self { elements: (0 as u64..size).collect() , rng: Box::new(StdRng::seed_from_u64(seed)), }
    }

    pub fn new(size: u64) -> Self {
        Self {
            elements: (0..size).collect(),
            rng: Box::new(rng()),
        }
    }
}

impl Iterator for RandomOrderIterator {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.elements.is_empty() {
            None
        } else {
            let rand_idx = self.rng.random_range(0..self.elements.len());
            Some(self.elements.swap_remove(rand_idx))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.elements.len(), Some(self.elements.len()))
    }
}

const BLOCK_SIZE: u64 = 512;
const RAND_SEED: u64 = 1234;
const NUM_THREADS: usize = 4;

fn main() {

    let args: Vec<String> = env::args().collect();
    let (filepath, block_size, num_threads) = if args.len() >= 4 {
        let m = args[2].parse::<u64>().unwrap_or(BLOCK_SIZE);
        let t = args[3].parse::<usize>().unwrap_or(NUM_THREADS);
        (args[1].clone(), m, t)
    } else if args.len() == 3 {
        let m = args[2].parse::<u64>().unwrap_or(BLOCK_SIZE);
        (args[1].clone(), m, NUM_THREADS)
    } else if args.len() == 2 {
        (args[1].clone(), BLOCK_SIZE, NUM_THREADS)
    } else {
        println!("Usage: cargo run --release --bin latency <ścieżka> [rozmiar_bloku] [wątki]");
        return;
    };

    let file_res = File::open(&filepath);
    if file_res.is_err() {
        eprintln!("File not found: {}", filepath);
        return;
    }
    let file = file_res.unwrap();

    let metadata = file.metadata();
    if metadata.is_err() {
        eprintln!("Error opening file: {}", filepath);
        return;
    }
    let total_size = metadata.unwrap().len();
    let num_blocks = (total_size + block_size - 1) / block_size;
    
    let block_indices: Vec<u64> = RandomOrderIterator::with_seed(num_blocks, RAND_SEED).collect();
    let shared_indices = Arc::new(Mutex::new(block_indices));

    let (sender, receiver) = mpsc::sync_channel::<Vec<u8>>(num_threads * 4);

    let start_total = std::time::Instant::now();
    let mut handles = Vec::new();
    for _ in 0..num_threads {
        let file_clone = File::open(&filepath).unwrap();
        let indices_clone = Arc::clone(&shared_indices);
        let sender_clone = sender.clone();

        handles.push(thread::spawn(move || {
            let mut thread_read_dur = std::time::Duration::ZERO;
            let mut thread_reads = 0u64;
            loop {
                let block_idx = {
                    let mut indices = indices_clone.lock().unwrap();
                    indices.pop()
                };

                let block_idx = match block_idx {
                    Some(idx) => idx,
                    None => break,
                };

                let offset = block_idx * block_size;
                let bytes_to_read = std::cmp::min(block_size, total_size - offset) as usize;
                let mut buffer = vec![0u8; bytes_to_read];

                let t0 = std::time::Instant::now();
                if let Err(err) = file_clone.read_exact_at(&mut buffer, offset) {
                    eprintln!("Error reading file: {}", err);
                    break;
                }
                thread_read_dur += t0.elapsed();
                thread_reads += 1;

                if sender_clone.send(buffer).is_err() {
                    break;
                }
            }
            (thread_reads, thread_read_dur)
        }));
    }
    drop(sender);

    let consumer = thread::spawn(move || {
        let mut res = vec![0u8; 16];
        for buffer in receiver {
            let block_hash = md5::compute(&buffer);
            sum_block_hash(&mut res, &block_hash);
        }
        res
    });

    let mut total_reads = 0u64;
    let mut total_read_dur = std::time::Duration::ZERO;
    for handle in handles {
        let (r_count, r_dur) = handle.join().unwrap();
        total_reads += r_count;
        total_read_dur += r_dur;
    }
    let result = consumer.join().unwrap();
    let elapsed_total = start_total.elapsed();

    let avg_read_us = if total_reads > 0 {
        (total_read_dur.as_secs_f64() * 1_000_000.0) / (total_reads as f64)
    } else {
        0.0
    };

    eprintln!(
        "czas_całk_s: {:.6}, liczba_odczytów: {}, śr_odczyt_µs: {:.3}",
        elapsed_total.as_secs_f64(),
        total_reads,
        avg_read_us
    );

    print!("{} 0x", filepath);
    for byte in &result {
        print!("{:02x}", byte);
    }
    println!("");

}
