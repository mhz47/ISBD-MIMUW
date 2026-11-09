use isbd_dbms::format::deserializer::BatchIterator;
use isbd_dbms::metrics;

use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: isbd-viewer <file_path>");
        process::exit(1);
    }

    let file_path = &args[1];
    println!("Loading file: {}", file_path);
    
    let batch_iter = BatchIterator::new(file_path)
        .expect("Failed to open file");

    let (int64_metrics, boolean_metrics, varchar_metrics) = 
        metrics::calculate_metrics_stream(batch_iter)
            .expect("Failed to calculate metrics");

    println!("\n--- INT64 Metrics ---");
    for m in &int64_metrics {
        println!("Column '{}': average = {:.4}", m.name, m.average);
    }

    println!("\n--- BOOLEAN Metrics ---");
    for m in &boolean_metrics {
        println!("Column '{}': binary entropy = {:.4}", m.name, m.entropy);
    }

    println!("\n--- VARCHAR Metrics ---");
    for m in &varchar_metrics {
        println!("Column '{}': character count = {}", m.name, m.char_count);
    }
}
