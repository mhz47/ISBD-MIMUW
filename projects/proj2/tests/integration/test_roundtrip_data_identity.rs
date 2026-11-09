use isbd_dbms::format::{Batch, ColumnBatch, Int64ColumnBatch, BooleanColumnBatch, VarcharColumnBatch};
use super::utils::{deserialize_file, serialize_batches};

/// Helper to compare two batches for exact data equality.
fn assert_batches_equal(input: &Batch, output: &Batch, batch_idx: usize) {
    assert_eq!(input.num_rows, output.num_rows,
        "Batch {}: row count mismatch: input={}, output={}",
        batch_idx, input.num_rows, output.num_rows);
    assert_eq!(input.columns.len(), output.columns.len(),
        "Batch {}: column count mismatch", batch_idx);

    for (i, (in_col, out_col)) in input.columns.iter().zip(output.columns.iter()).enumerate() {
        match (in_col, out_col) {
            (ColumnBatch::Int64(in_c), ColumnBatch::Int64(out_c)) => {
                assert_eq!(in_c.name, out_c.name, "Batch {} col {}: name mismatch", batch_idx, i);
                assert_eq!(in_c.values, out_c.values,
                    "Batch {} col {}: INT64 values mismatch", batch_idx, i);
            }
            (ColumnBatch::Boolean(in_c), ColumnBatch::Boolean(out_c)) => {
                assert_eq!(in_c.name, out_c.name, "Batch {} col {}: name mismatch", batch_idx, i);
                assert_eq!(in_c.values, out_c.values,
                    "Batch {} col {}: BOOLEAN values mismatch", batch_idx, i);
            }
            (ColumnBatch::Varchar(in_c), ColumnBatch::Varchar(out_c)) => {
                assert_eq!(in_c.name, out_c.name, "Batch {} col {}: name mismatch", batch_idx, i);
                assert_eq!(in_c.values, out_c.values,
                    "Batch {} col {}: VARCHAR values mismatch", batch_idx, i);
            }
            _ => panic!("Batch {} col {}: type mismatch", batch_idx, i),
        }
    }
}

#[test]
fn test_roundtrip_small_batches_data_identity() {
    let batch_size = 500;
    let num_batches = 3;
    let total_rows = batch_size * num_batches;

    let int64_vals: Vec<i64> = (0..total_rows).map(|i| i as i64 * 10 - 5).collect();
    let bool_vals: Vec<bool> = (0..total_rows).map(|i| i % 3 != 0).collect();
    let varchar_vals: Vec<String> = (0..total_rows).map(|i| format!("item_{}", i)).collect();

    let mut batches = Vec::new();
    for i in 0..num_batches {
        let start = i * batch_size;
        let end = start + batch_size;
        batches.push(Batch {
            columns: vec![
                ColumnBatch::Int64(Int64ColumnBatch {
                    name: "id".to_string(),
                    values: int64_vals[start..end].to_vec(),
                }),
                ColumnBatch::Boolean(BooleanColumnBatch {
                    name: "active".to_string(),
                    values: bool_vals[start..end].to_vec(),
                }),
                ColumnBatch::Varchar(VarcharColumnBatch {
                    name: "name".to_string(),
                    values: varchar_vals[start..end].to_vec(),
                }),
            ],
            num_rows: batch_size,
        });
    }

    let path = "/tmp/test_small_batches_identity.isbd";
    serialize_batches(batches.clone(), path).expect("Serialization failed");
    let deserialized = deserialize_file(path).expect("Deserialization failed");

    // With page_row_counts, each 500-row batch becomes 1 page, so we expect 3 pages.
    assert_eq!(deserialized.len(), num_batches,
        "Expected {} batches, got {}", num_batches, deserialized.len());

    for (i, (input, output)) in batches.iter().zip(deserialized.iter()).enumerate() {
        assert_batches_equal(input, output, i);
    }

    std::fs::remove_file(path).ok();
}

#[test]
fn test_roundtrip_single_row_data_identity() {
    let batch = Batch {
        columns: vec![
            ColumnBatch::Int64(Int64ColumnBatch {
                name: "id".to_string(),
                values: vec![42],
            }),
            ColumnBatch::Boolean(BooleanColumnBatch {
                name: "active".to_string(),
                values: vec![true],
            }),
            ColumnBatch::Varchar(VarcharColumnBatch {
                name: "name".to_string(),
                values: vec!["Solo".to_string()],
            }),
        ],
        num_rows: 1,
    };

    let batches = vec![batch.clone()];
    let path = "/tmp/test_single_row_identity.isbd";
    serialize_batches(batches, path).expect("Serialization failed");
    let deserialized = deserialize_file(path).expect("Deserialization failed");

    assert_eq!(deserialized.len(), 1);
    assert_batches_equal(&batch, &deserialized[0], 0);

    std::fs::remove_file(path).ok();
}

#[test]
fn test_roundtrip_large_batch_data_identity() {
    let batch_size = 10_000;

    let int64_vals: Vec<i64> = (0..batch_size).map(|i| i as i64).collect();
    let bool_vals: Vec<bool> = (0..batch_size).map(|i| i % 2 == 0).collect();
    let varchar_vals: Vec<String> = (0..batch_size).map(|i| format!("row_{}", i)).collect();

    let batch = Batch {
        columns: vec![
            ColumnBatch::Int64(Int64ColumnBatch {
                name: "id".to_string(),
                values: int64_vals.clone(),
            }),
            ColumnBatch::Boolean(BooleanColumnBatch {
                name: "active".to_string(),
                values: bool_vals.clone(),
            }),
            ColumnBatch::Varchar(VarcharColumnBatch {
                name: "name".to_string(),
                values: varchar_vals.clone(),
            }),
        ],
        num_rows: batch_size,
    };

    let batches = vec![batch.clone()];
    let path = "/tmp/test_large_batch_identity.isbd";
    serialize_batches(batches, path).expect("Serialization failed");
    let deserialized = deserialize_file(path).expect("Deserialization failed");

    // 10,000 rows split into pages of 1024: 1024*9 = 9216, remainder 784 -> 10 pages total.
    let expected_pages = 10;
    assert_eq!(deserialized.len(), expected_pages,
        "Expected {} pages, got {}", expected_pages, deserialized.len());

    // Reconstruct the full data from deserialized pages and compare with original.
    let mut all_int64 = Vec::new();
    let mut all_bool = Vec::new();
    let mut all_varchar = Vec::new();

    for page_batch in &deserialized {
        match &page_batch.columns[0] {
            ColumnBatch::Int64(col) => all_int64.extend_from_slice(&col.values),
            _ => panic!("Expected Int64 column"),
        }
        match &page_batch.columns[1] {
            ColumnBatch::Boolean(col) => all_bool.extend_from_slice(&col.values),
            _ => panic!("Expected Boolean column"),
        }
        match &page_batch.columns[2] {
            ColumnBatch::Varchar(col) => all_varchar.extend_from_slice(&col.values),
            _ => panic!("Expected Varchar column"),
        }
    }

    assert_eq!(all_int64, int64_vals, "INT64 data mismatch after roundtrip");
    assert_eq!(all_bool, bool_vals, "BOOLEAN data mismatch after roundtrip");
    assert_eq!(all_varchar, varchar_vals, "VARCHAR data mismatch after roundtrip");

    std::fs::remove_file(path).ok();
}

#[test]
fn test_roundtrip_mixed_batch_sizes_data_identity() {
    let batch_sizes = vec![1, 500, 1024, 2000, 100];
    let mut global_idx = 0;
    let mut all_input_batches = Vec::new();

    for size in &batch_sizes {
        let int64_vals: Vec<i64> = (0..*size).map(|i| (global_idx + i) as i64).collect();
        let bool_vals: Vec<bool> = (0..*size).map(|i| (global_idx + i) % 2 == 0).collect();
        let varchar_vals: Vec<String> = (0..*size).map(|i| format!("v_{}", global_idx + i)).collect();

        all_input_batches.push(Batch {
            columns: vec![
                ColumnBatch::Int64(Int64ColumnBatch {
                    name: "id".to_string(),
                    values: int64_vals.clone(),
                }),
                ColumnBatch::Boolean(BooleanColumnBatch {
                    name: "active".to_string(),
                    values: bool_vals.clone(),
                }),
                ColumnBatch::Varchar(VarcharColumnBatch {
                    name: "name".to_string(),
                    values: varchar_vals.clone(),
                }),
            ],
            num_rows: *size,
        });
        global_idx += size;
    }

    let total_rows = global_idx;
    let path = "/tmp/test_mixed_batches_identity.isbd";
    serialize_batches(all_input_batches.clone(), path).expect("Serialization failed");
    let deserialized = deserialize_file(path).expect("Deserialization failed");

    // Reconstruct full data from deserialized pages.
    let mut all_int64 = Vec::new();
    let mut all_bool = Vec::new();
    let mut all_varchar = Vec::new();

    for page_batch in &deserialized {
        match &page_batch.columns[0] {
            ColumnBatch::Int64(col) => all_int64.extend_from_slice(&col.values),
            _ => panic!("Expected Int64 column"),
        }
        match &page_batch.columns[1] {
            ColumnBatch::Boolean(col) => all_bool.extend_from_slice(&col.values),
            _ => panic!("Expected Boolean column"),
        }
        match &page_batch.columns[2] {
            ColumnBatch::Varchar(col) => all_varchar.extend_from_slice(&col.values),
            _ => panic!("Expected Varchar column"),
        }
    }

    // Compare with original data.
    let mut expected_int64 = Vec::new();
    let mut expected_bool = Vec::new();
    let mut expected_varchar = Vec::new();
    for batch in &all_input_batches {
        match &batch.columns[0] {
            ColumnBatch::Int64(col) => expected_int64.extend_from_slice(&col.values),
            _ => unreachable!(),
        }
        match &batch.columns[1] {
            ColumnBatch::Boolean(col) => expected_bool.extend_from_slice(&col.values),
            _ => unreachable!(),
        }
        match &batch.columns[2] {
            ColumnBatch::Varchar(col) => expected_varchar.extend_from_slice(&col.values),
            _ => unreachable!(),
        }
    }

    assert_eq!(all_int64, expected_int64, "INT64 data mismatch after roundtrip with mixed batch sizes");
    assert_eq!(all_bool, expected_bool, "BOOLEAN data mismatch after roundtrip with mixed batch sizes");
    assert_eq!(all_varchar, expected_varchar, "VARCHAR data mismatch after roundtrip with mixed batch sizes");

    // Verify total row count.
    let deserialized_total: usize = deserialized.iter().map(|b| b.num_rows).sum();
    assert_eq!(deserialized_total, total_rows,
        "Total row count mismatch: expected {}, got {}", total_rows, deserialized_total);

    std::fs::remove_file(path).ok();
}
