use isbd_dbms::format::{Batch, ColumnBatch, Int64ColumnBatch, BooleanColumnBatch, VarcharColumnBatch, serializer::serialize_to_file, BatchIterator};
use isbd_dbms::metrics::calculate_metrics_stream;

#[test]
fn test_streaming_metrics_multiple_batches() {
    // Create 2500 rows -> 3 batches (1024 + 1024 + 452).
    let num_rows = 2500;
    let columns = vec![
        ColumnBatch::Int64(Int64ColumnBatch {
            name: "id".to_string(),
            values: (1..=num_rows as i64).collect(),
        }),
        ColumnBatch::Boolean(BooleanColumnBatch {
            name: "flag".to_string(),
            values: (0..num_rows).map(|i| i % 3 == 0).collect(),
        }),
        ColumnBatch::Varchar(VarcharColumnBatch {
            name: "text".to_string(),
            values: (0..num_rows).map(|_| "abc".to_string()).collect(),
        }),
    ];

    let batch = Batch { columns, num_rows };
    let tmp = "/tmp/test_streaming_metrics.isbd";
    let iter = std::iter::once(Ok(batch));
    serialize_to_file(iter, tmp).unwrap();

    // Use streaming metrics.
    let iter = BatchIterator::new(tmp).unwrap();
    let (int64_metrics, boolean_metrics, varchar_metrics) = calculate_metrics_stream(iter).unwrap();

    // Verify INT64: average of 1..2500 = (1+2500)/2 = 1250.5.
    assert_eq!(int64_metrics.len(), 1);
    assert_eq!(int64_metrics[0].name, "id");
    assert!((int64_metrics[0].average - 1250.5).abs() < 0.01);

    // Verify BOOLEAN: every 3rd is true -> p = 833/2500 = 0.3332.
    assert_eq!(boolean_metrics.len(), 1);
    assert_eq!(boolean_metrics[0].name, "flag");
    let p: f64 = 833.0 / 2500.0;
    let expected_entropy = -p * p.log2() - (1.0 - p) * (1.0 - p).log2();
    assert!((boolean_metrics[0].entropy - expected_entropy).abs() < 0.01);

    // Verify VARCHAR: 2500 * 3 = 7500 characters
    assert_eq!(varchar_metrics.len(), 1);
    assert_eq!(varchar_metrics[0].name, "text");
    assert_eq!(varchar_metrics[0].char_count, 7500);

    std::fs::remove_file(tmp).ok();
}

#[test]
fn test_streaming_metrics_single_batch() {
    let columns = vec![
        ColumnBatch::Int64(Int64ColumnBatch {
            name: "v".to_string(),
            values: vec![10, 20, 30],
        }),
    ];
    let batch = Batch { columns, num_rows: 3 };
    let tmp = "/tmp/test_streaming_single.isbd";
    let iter = std::iter::once(Ok(batch));
    serialize_to_file(iter, tmp).unwrap();

    let iter = BatchIterator::new(tmp).unwrap();
    let (int64_metrics, _, _) = calculate_metrics_stream(iter).unwrap();

    assert_eq!(int64_metrics.len(), 1);
    assert!((int64_metrics[0].average - 20.0).abs() < 0.01);

    std::fs::remove_file(tmp).ok();
}

#[test]
fn test_streaming_metrics_multiple_same_type_columns() {
    let columns = vec![
        ColumnBatch::Int64(Int64ColumnBatch {
            name: "a".to_string(),
            values: vec![10, 20, 30],
        }),
        ColumnBatch::Int64(Int64ColumnBatch {
            name: "b".to_string(),
            values: vec![100, 200, 300],
        }),
        ColumnBatch::Varchar(VarcharColumnBatch {
            name: "x".to_string(),
            values: vec!["ab".to_string(), "cd".to_string(), "ef".to_string()],
        }),
        ColumnBatch::Varchar(VarcharColumnBatch {
            name: "y".to_string(),
            values: vec!["hello".to_string(), "world".to_string(), "foo".to_string()],
        }),
    ];
    let batch = Batch { columns, num_rows: 3 };
    let tmp = "/tmp/test_streaming_multi_col.isbd";
    let iter = std::iter::once(Ok(batch));
    serialize_to_file(iter, tmp).unwrap();

    let iter = BatchIterator::new(tmp).unwrap();
    let (int64_metrics, _, varchar_metrics) = calculate_metrics_stream(iter).unwrap();

    // Two INT64 columns with different averages.
    assert_eq!(int64_metrics.len(), 2);
    let a = int64_metrics.iter().find(|m| m.name == "a").unwrap();
    let b = int64_metrics.iter().find(|m| m.name == "b").unwrap();
    assert!((a.average - 20.0).abs() < 0.01);
    assert!((b.average - 200.0).abs() < 0.01);

    // Two VARCHAR columns with different char counts.
    assert_eq!(varchar_metrics.len(), 2);
    let x = varchar_metrics.iter().find(|m| m.name == "x").unwrap();
    let y = varchar_metrics.iter().find(|m| m.name == "y").unwrap();
    assert_eq!(x.char_count, 6);  // "ab" + "cd" + "ef" = 6
    assert_eq!(y.char_count, 13); // "hello" + "world" + "foo" = 13

    std::fs::remove_file(tmp).ok();
}
