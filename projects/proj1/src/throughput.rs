use std::env;
use std::fs::File;

mod hash;
use hash::sum_block_hash;

const BLOCK_SIZE: u64 = 512;

fn main() {

    let args: Vec<String> = env::args().collect();
    let (filepath, block_size) = if args.len() >= 3 {
        let m = args[2].parse::<u64>().unwrap_or(BLOCK_SIZE);
        (args[1].clone(), m)
    } else if args.len() == 2 {
        (args[1].clone(), BLOCK_SIZE)
    } else {
        println!("Usage: cargo run --release --bin throughput <ścieżka> [rozmiar_bloku]");
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

    let mut result = vec![0u8; 16];
    let start_total = std::time::Instant::now();
    if total_size > 0 {
        // Mapowanie pliku bezpośrednio do przestrzeni adresowej procesu za pomocą wywołania systemowego mmap(2).
        // Blok unsafe jest wymagany przez bibliotekę memmap2, gdyż niespójność danych z zewnętrznej modyfikacji
        // pliku podczas działania programu mogłaby prowadzić do niezdefiniowanego zachowania (Undefined Behavior).
        let mmap = unsafe { memmap2::Mmap::map(&file).unwrap() };
        let num_blocks = (total_size + block_size - 1) / block_size;

        for block_idx in 0..num_blocks {
            let offset = (block_idx * block_size) as usize;
            let end = std::cmp::min(offset + block_size as usize, total_size as usize);
            let target_buf = &mmap[offset..end];

            // compute
            let block_hash = md5::compute(target_buf);
            sum_block_hash(&mut result, &block_hash);
        }
    }

    let elapsed_total = start_total.elapsed();
    eprintln!("czas_całk_s: {:.6}", elapsed_total.as_secs_f64());

    print!("{} 0x", filepath);
    for byte in &result {
        print!("{:02x}", byte);
    }
    println!("");

}