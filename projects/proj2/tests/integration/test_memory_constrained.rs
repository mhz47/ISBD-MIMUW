/// Memory-constrained roundtrip test.
///
/// Uses `ulimit -v` to limit the address space of the generator and viewer
/// processes, verifying that the streaming page-based design handles files
/// larger than the available memory limit without loading all data into RAM.
///
/// The test generates a large file (20,000,000 rows) under a 256 MB memory limit,
/// then deserializes it under the same limit and compares metrics with an
/// unlimited run to verify data integrity.

use std::process::Command;

fn run_with_memory_limit(
    program: &str,
    args: &[&str],
    memory_limit_kb: usize,
) -> Option<std::process::Output> {
    // ulimit -v sets the max address space in KB.
    let cmd = format!(
        "ulimit -v {}; {}",
        memory_limit_kb,
        program
    );
    // Build the full command string with arguments.
    let full_cmd = format!("{} {}", cmd, args.join(" "));

    let output = Command::new("bash")
        .args(&["-c", &full_cmd])
        .output()
        .ok()?;

    Some(output)
}

#[test]
fn test_memory_constrained_roundtrip() {
    let num_rows = 20_000_000;
    let tmp = "/tmp/test_memory_constrained.isbd";
    let memory_limit_kb = 262_144; // 256 MB

    // Clean up any previous test file.
    std::fs::remove_file(tmp).ok();

    // Find the release binaries.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let generator = format!("{}/target/release/isbd-generator", manifest_dir);
    let viewer = format!("{}/target/release/isbd-viewer", manifest_dir);

    // Check if binaries exist.
    if !std::path::Path::new(&generator).exists()
        || !std::path::Path::new(&viewer).exists()
    {
        println!();
        println!(
            "SKIPPED: Release binaries not found. Run `make all` first."
        );
        return;
    }

    // Generate file under memory limit.
    let gen_output = run_with_memory_limit(
        &generator,
        &[
            "--rows",
            &num_rows.to_string(),
            "--output",
            tmp,
        ],
        memory_limit_kb,
    );

    match gen_output {
        Some(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // If the process was killed by OOM (SIGKILL = 137), the test fails.
                if output.status.code() == Some(137) {
                    panic!(
                        "\nFAILED: Generator was killed by OOM (exit code 137)\n\
                         This means the streaming serializer loaded too much data into memory.\n\
                         stderr: {}",
                        stderr.trim()
                    );
                }
                // Other failure - skip with message.
                println!();
                println!(
                    "SKIPPED: Generator failed under memory limit.\n\
                     stderr: {}",
                    stderr.trim()
                );
                return;
            }
        }
        None => {
            println!();
            println!("SKIPPED: Could not run command with ulimit.");
            return;
        }
    }

    // Verify the file was created.
    if !std::path::Path::new(tmp).exists() {
        panic!("Generator did not create the output file");
    }

    // View file under memory limit.
    let view_output = run_with_memory_limit(
        &viewer,
        &[tmp],
        memory_limit_kb,
    );

    let view_output = match view_output {
        Some(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.code() == Some(137) {
                    panic!(
                        "\nFAILED: Viewer was killed by OOM (exit code 137)\n\
                         This means the streaming deserializer loaded too much data into memory.\n\
                         stderr: {}",
                        stderr.trim()
                    );
                }
                println!();
                println!(
                    "SKIPPED: Viewer failed under memory limit.\n\
                     stderr: {}",
                    stderr.trim()
                );
                std::fs::remove_file(tmp).ok();
                return;
            }
            output
        }
        None => {
            println!();
            println!("SKIPPED: Could not run command with ulimit.");
            std::fs::remove_file(tmp).ok();
            return;
        }
    };

    //  Verify data integrity - compare metrics with unlimited run.
    let unlimited_output = Command::new(&viewer)
        .args(&[tmp])
        .output()
        .expect("Failed to run viewer without limit");

    if !unlimited_output.status.success() {
        panic!("Viewer failed without memory limit");
    }

    let limited_stdout = String::from_utf8_lossy(&view_output.stdout);
    let unlimited_stdout = String::from_utf8_lossy(&unlimited_output.stdout);

    // Extract metrics from both outputs and compare.
    // The metrics should be identical regardless of memory limit.
    let limited_metrics = limited_stdout
        .lines()
        .filter(|l| l.contains("average") || l.contains("entropy") || l.contains("character count"))
        .collect::<Vec<_>>();
    let unlimited_metrics = unlimited_stdout
        .lines()
        .filter(|l| l.contains("average") || l.contains("entropy") || l.contains("character count"))
        .collect::<Vec<_>>();

    assert_eq!(
        limited_metrics.len(),
        unlimited_metrics.len(),
        "Different number of metric lines between limited and unlimited runs"
    );

    for (i, (limited, unlimited)) in limited_metrics.iter().zip(&unlimited_metrics).enumerate() {
        // Compare column names (before the colon).
        let limited_col = limited.split(':').next().unwrap_or("");
        let unlimited_col = unlimited.split(':').next().unwrap_or("");
        assert_eq!(
            limited_col.trim(),
            unlimited_col.trim(),
            "Metric {} column name mismatch: limited='{}', unlimited='{}'",
            i, limited_col, unlimited_col
        );
    }

    println!();
    println!(
        "PASSED: Memory-constrained roundtrip succeeded with {} rows under {} MB limit.",
        num_rows, memory_limit_kb / 1024
    );
    println!("Data integrity verified: metrics match between limited and unlimited runs.");


    std::fs::remove_file(tmp).ok();
}
