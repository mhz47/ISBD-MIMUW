use isbd_dbms::format::{Batch, ColumnBatch, Int64ColumnBatch, BooleanColumnBatch, serializer::serialize_to_file};

/// Test that serializing inconsistent batches returns an error.
#[test]
fn test_serialize_inconsistent_batch_count() {
    let batches = vec![
        Batch {
            columns: vec![ColumnBatch::Int64(Int64ColumnBatch {
                name: "v".to_string(),
                values: vec![1, 2, 3],
            })],
            num_rows: 3,
        },
        Batch {
            columns: vec![
                ColumnBatch::Int64(Int64ColumnBatch {
                    name: "v".to_string(),
                    values: vec![4, 5, 6],
                }),
                ColumnBatch::Boolean(BooleanColumnBatch {
                    name: "f".to_string(),
                    values: vec![true, false, true],
                }),
            ],
            num_rows: 3,
        },
    ];

    let iter = batches.into_iter().map(Ok);
    let result = serialize_to_file(iter, "/tmp/test_inconsistent_count.isbd");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("columns"));
}

/// Test that serializing batches with mismatched column types returns an error.
#[test]
fn test_serialize_inconsistent_column_types() {
    let batches = vec![
        Batch {
            columns: vec![ColumnBatch::Int64(Int64ColumnBatch {
                name: "v".to_string(),
                values: vec![1, 2, 3],
            })],
            num_rows: 3,
        },
        Batch {
            columns: vec![ColumnBatch::Boolean(BooleanColumnBatch {
                name: "v".to_string(),
                values: vec![true, false, true],
            })],
            num_rows: 3,
        },
    ];

    let iter = batches.into_iter().map(Ok);
    let result = serialize_to_file(iter, "/tmp/test_inconsistent_types.isbd");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("type"));
}
