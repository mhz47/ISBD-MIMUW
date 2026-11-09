use isbd_dbms::format::{Batch, ColumnBatch, Int64ColumnBatch, BooleanColumnBatch, VarcharColumnBatch, serializer::serialize_to_file, BatchIterator};

#[test]
fn test_page_independence() {
    // Create 3000 rows -> 3 pages (1024 + 1024 + 952).
    let num_rows = 3000;
    let columns = vec![
        ColumnBatch::Int64(Int64ColumnBatch {
            name: "id".to_string(),
            values: (0..num_rows as i64).collect(),
        }),
        ColumnBatch::Boolean(BooleanColumnBatch {
            name: "flag".to_string(),
            values: (0..num_rows).map(|i| i % 2 == 0).collect(),
        }),
        ColumnBatch::Varchar(VarcharColumnBatch {
            name: "text".to_string(),
            values: (0..num_rows).map(|i| format!("row_{}", i)).collect(),
        }),
    ];

    let batch = Batch {
        columns: columns.clone(),
        num_rows,
    };
    let iter = std::iter::once(Ok(batch));

    let tmp = "/tmp/test_page_independence.isbd";
    serialize_to_file(iter, tmp).unwrap();

    // Open iterator and skip directly to the last batch (page 2).
    let mut iter = BatchIterator::new(tmp).unwrap();

    // Skip first two batches.
    let _batch0 = iter.next().unwrap().unwrap();
    let _batch1 = iter.next().unwrap().unwrap();

    // Third batch should have 952 rows (3000 - 2*1024).
    let batch2 = iter.next().unwrap().unwrap();
    assert_eq!(batch2.num_rows, 952);

    // Verify data in the last page.
    if let ColumnBatch::Int64(col) = &batch2.columns[0] {
        let expected: Vec<i64> = (2048..3000).collect();
        assert_eq!(col.values, expected);
    } else {
        panic!("Expected Int64 column");
    }

    // No more batches.
    assert!(iter.next().is_none());

    std::fs::remove_file(tmp).ok();
}

