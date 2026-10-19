use std::env;
use std::fs::File;
use std::os::unix::fs::FileExt;

mod hash;
use hash::sum_block_hash;

const BLOCK_SIZE: u64 = 512;

fn main() {

    let args: Vec<String> = env::args().collect();
    let block_size = if args.len() >= 3 {
        args[2].parse::<u64>().unwrap_or(BLOCK_SIZE)
    } else if args.len() == 2 {
        BLOCK_SIZE
    } else {
        println!("Usage: cargo run --release --bin throughput <ścieżka> [rozmiar_bloku]");
        return;
    };

    let file_res = File::open(args[1].clone());
    if file_res.is_err() {
        eprintln!("File not found: {}", args[1]);
        return;
    }
    let file = file_res.unwrap();

    let metadata = file.metadata();
    if metadata.is_err() {
        eprintln!("Error opening file: {}", args[1]);
        return;
    }
    let total_size = metadata.unwrap().len();
    let num_blocks = (total_size + block_size - 1) / block_size;

    let mut buffer = vec![0u8; block_size as usize];
    let mut result = vec![0u8; 16];

    let start_total = std::time::Instant::now();
    let mut read_time = std::time::Duration::ZERO;

    for block_idx in 0..num_blocks
    {
        let offset = block_idx * block_size;
        let bytes_to_read = std::cmp::min(block_size, total_size - offset) as usize;

        // read
        let target_buf = &mut buffer[..bytes_to_read];
        let t0 = std::time::Instant::now();
        let err = file.read_exact_at(target_buf, offset);
        if err.is_err() {
            eprintln!("Error reading file: {}", err.unwrap_err());
            return;
        }
        read_time += t0.elapsed();

        // compute
        let block_hash = md5::compute(target_buf);
        sum_block_hash(&mut result, &block_hash);
    }
    let elapsed_total = start_total.elapsed();

    eprintln!(
        "czas_całk_s: {:.6}, czas_odczyt_s: {:.6}",
        elapsed_total.as_secs_f64(),
        read_time.as_secs_f64()
    );

    print!("{} 0x", args[1].clone());
    for byte in &result {
        print!("{:02x}", byte);
    }
    println!("");

}