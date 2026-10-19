use std::env;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::sync::mpsc;
use std::thread;

mod hash;
use hash::sum_block_hash;
use hash::stretch_md5;

const BLOCK_SIZE: u64 = 1024 * 1024;
const K: u64 = 1000;

fn main() {

    let args: Vec<String> = env::args().collect();
    let (filepath, block_size, k) = if args.len() >= 4 {
        let m = args[2].parse::<u64>().unwrap_or(BLOCK_SIZE);
        let k_val = args[3].parse::<u64>().unwrap_or(K);
        (args[1].clone(), m, k_val)
    } else if args.len() == 3 {
        let m = args[2].parse::<u64>().unwrap_or(BLOCK_SIZE);
        (args[1].clone(), m, K)
    } else if args.len() == 2 {
        (args[1].clone(), BLOCK_SIZE, K)
    } else {
        println!("Usage: cargo run --release --bin pipeline <ścieżka> [rozmiar_bloku] [iteracje]");
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

    let (sender, receiver) = mpsc::sync_channel::<Vec<u8>>(4);

    let start_total = std::time::Instant::now();

    let producer = thread::spawn(move || {
        let mut read_dur = std::time::Duration::ZERO;
        for block_idx in 0..num_blocks {
            let offset = block_idx * block_size;
            let bytes_to_read = std::cmp::min(block_size, total_size - offset) as usize;
            let mut buffer = vec![0u8; bytes_to_read];

            let t0 = std::time::Instant::now();
            if let Err(err) = file.read_exact_at(&mut buffer, offset) {
                eprintln!("Error reading file: {}", err);
                break;
            }
            read_dur += t0.elapsed();

            if sender.send(buffer).is_err() {
                break;
            }
        }
        read_dur
    });

    let consumer = thread::spawn(move || {
        let mut res = vec![0u8; 16];
        let mut compute_dur = std::time::Duration::ZERO;
        for buffer in receiver {
            let t0 = std::time::Instant::now();
            let block_hash = stretch_md5(&buffer, k);
            sum_block_hash(&mut res, &block_hash);
            compute_dur += t0.elapsed();
        }
        (res, compute_dur)
    });

    let read_dur = producer.join().unwrap();
    let (result, compute_dur) = consumer.join().unwrap();
    let elapsed_total = start_total.elapsed();

    eprintln!(
        "czas_całk_s: {:.6}, czas_odczyt_s: {:.6}, czas_oblicz_s: {:.6}",
        elapsed_total.as_secs_f64(),
        read_dur.as_secs_f64(),
        compute_dur.as_secs_f64()
    );

    print!("{} 0x", filepath);
    for byte in &result {
        print!("{:02x}", byte);
    }
    println!("");

}