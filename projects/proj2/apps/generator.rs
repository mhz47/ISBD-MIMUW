use isbd_dbms::format::{Batch, ColumnBatch, Int64ColumnBatch, BooleanColumnBatch, VarcharColumnBatch, PAGE_SIZE};
use isbd_dbms::format::serializer;
use rand::Rng;
use std::env;
use std::path::Path;

/// Generate a random string of length between `min_len` and `max_len`.
fn random_string(rng: &mut impl Rng, min_len: usize, max_len: usize) -> String {
    let len = rng.gen_range(min_len..=max_len);
    let chars: Vec<char> = (0..len)
        .map(|_| {
            let c = rng.gen_range(b'a'..=b'z');
            c as char
        })
        .collect();
    chars.into_iter().collect()
}

/// Parse command-line arguments.
/// Usage: isbd-generator [--rows N] [--output PATH]
fn parse_args() -> (usize, String) {
    let args: Vec<String> = env::args().collect();
    let mut num_rows: usize = 10;
    let mut output_path = "data/sample.isbd".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--rows" => {
                i += 1;
                if i < args.len() {
                    num_rows = args[i]
                        .parse::<usize>()
                        .unwrap_or_else(|_| {
                            eprintln!("Error: --rows requires a valid integer, got '{}'", args[i]);
                            std::process::exit(1);
                        });
                } else {
                    eprintln!("Error: --rows requires a value");
                    std::process::exit(1);
                }
            }
            "--output" => {
                i += 1;
                if i < args.len() {
                    output_path = args[i].clone();
                } else {
                    eprintln!("Error: --output requires a file path");
                    std::process::exit(1);
                }
            }
            unknown => {
                eprintln!("Error: unknown argument '{}'", unknown);
                eprintln!("Usage: isbd-generator [--rows N] [--output PATH]");
                std::process::exit(1);
            }
        }
        i += 1;
    }

    (num_rows, output_path)
}

fn main() {
    let (num_rows, output_path) = parse_args();

    // Ensure output directory exists.
    if let Some(parent) = Path::new(&output_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                eprintln!("Error creating directory {}: {}", parent.display(), e);
                std::process::exit(1);
            });
        }
    }

    let mut row_offset = 0;
    let mut current_int64: i64 = rand::thread_rng().gen_range(0..1000);
    let mut rng = rand::thread_rng();

    let batch_stream = std::iter::from_fn(move || {
        if row_offset >= num_rows {
            return None;
        }

        let remaining = num_rows - row_offset;
        let batch_size = std::cmp::min(PAGE_SIZE, remaining);

        // Generate INT64 values.
        let int64_values: Vec<i64> = (0..batch_size)
            .map(|_| {
                let delta: i64 = rng.gen_range(-10..=20);
                current_int64 = current_int64.wrapping_add(delta);
                current_int64
            })
            .collect();

        // Generate random BOOLEAN values.
        let bool_values: Vec<bool> = (0..batch_size)
            .map(|_| rng.gen())
            .collect();

        // Generate random VARCHAR values for column "name" (short strings, 3-12 chars).
        let name_values: Vec<String> = (0..batch_size)
            .map(|_| random_string(&mut rng, 3, 12))
            .collect();

        // Generate random VARCHAR values for column "description" (longer strings, 10-50 chars).
        let desc_values: Vec<String> = (0..batch_size)
            .map(|_| random_string(&mut rng, 10, 50))
            .collect();

        row_offset += batch_size;

        Some(Ok(Batch {
            columns: vec![
                ColumnBatch::Int64(Int64ColumnBatch {
                    name: "id".to_string(),
                    values: int64_values,
                }),
                ColumnBatch::Boolean(BooleanColumnBatch {
                    name: "active".to_string(),
                    values: bool_values,
                }),
                ColumnBatch::Varchar(VarcharColumnBatch {
                    name: "name".to_string(),
                    values: name_values,
                }),
                ColumnBatch::Varchar(VarcharColumnBatch {
                    name: "description".to_string(),
                    values: desc_values,
                }),
            ],
            num_rows: batch_size,
        }))
    });

    serializer::serialize_to_file(batch_stream, &output_path)
        .expect("Failed to serialize file");

    let num_batches = (num_rows + PAGE_SIZE - 1) / PAGE_SIZE;
    println!(
        "Generated file: {} ({} rows in {} batch(es), 1 INT64 + 1 BOOLEAN + 2 VARCHAR)",
        output_path, num_rows, num_batches
    );
}
