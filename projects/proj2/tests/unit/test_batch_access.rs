use isbd_dbms::format::{Batch, ColumnBatch, Int64ColumnBatch, BooleanColumnBatch, VarcharColumnBatch, CellValue};

#[test]
fn test_batch_get_set_int64() {
    let batch = Batch {
        columns: vec![
            ColumnBatch::Int64(Int64ColumnBatch {
                name: "v".to_string(),
                values: vec![10, 20, 30],
            }),
        ],
        num_rows: 3,
    };

    assert_eq!(batch.get(0, 0), Some(CellValue::Int64(10)));
    assert_eq!(batch.get(0, 1), Some(CellValue::Int64(20)));
    assert_eq!(batch.get(0, 2), Some(CellValue::Int64(30)));
    assert_eq!(batch.get(0, 3), None); // out of bounds
    assert_eq!(batch.get(1, 0), None); // out of bounds
}

#[test]
fn test_batch_get_set_boolean() {
    let batch = Batch {
        columns: vec![
            ColumnBatch::Boolean(BooleanColumnBatch {
                name: "f".to_string(),
                values: vec![true, false, true],
            }),
        ],
        num_rows: 3,
    };

    assert_eq!(batch.get(0, 0), Some(CellValue::Boolean(true)));
    assert_eq!(batch.get(0, 1), Some(CellValue::Boolean(false)));
}

#[test]
fn test_batch_get_set_varchar() {
    let batch = Batch {
        columns: vec![
            ColumnBatch::Varchar(VarcharColumnBatch {
                name: "s".to_string(),
                values: vec!["hello".to_string(), "world".to_string()],
            }),
        ],
        num_rows: 2,
    };

    assert_eq!(batch.get(0, 0), Some(CellValue::Varchar("hello".to_string())));
    assert_eq!(batch.get(0, 1), Some(CellValue::Varchar("world".to_string())));
}

#[test]
fn test_batch_set_int64() {
    let mut batch = Batch {
        columns: vec![
            ColumnBatch::Int64(Int64ColumnBatch {
                name: "v".to_string(),
                values: vec![10, 20, 30],
            }),
        ],
        num_rows: 3,
    };

    batch.set(0, 1, CellValue::Int64(99)).unwrap();
    if let ColumnBatch::Int64(col) = &batch.columns[0] {
        assert_eq!(col.values, vec![10, 99, 30]);
    } else {
        panic!("Expected Int64 column");
    }
}

#[test]
fn test_batch_set_varchar() {
    let mut batch = Batch {
        columns: vec![
            ColumnBatch::Varchar(VarcharColumnBatch {
                name: "s".to_string(),
                values: vec!["a".to_string(), "b".to_string()],
            }),
        ],
        num_rows: 2,
    };

    batch.set(0, 0, CellValue::Varchar("replaced".to_string())).unwrap();
    if let ColumnBatch::Varchar(col) = &batch.columns[0] {
        assert_eq!(col.values, vec!["replaced".to_string(), "b".to_string()]);
    } else {
        panic!("Expected Varchar column");
    }
}

#[test]
fn test_batch_concatenation_simulation() {
    let mut batch = Batch {
        columns: vec![
            ColumnBatch::Varchar(VarcharColumnBatch {
                name: "s1".to_string(),
                values: vec!["Hello".to_string(), "Foo".to_string()],
            }),
            ColumnBatch::Varchar(VarcharColumnBatch {
                name: "s2".to_string(),
                values: vec!["World".to_string(), "Bar".to_string()],
            }),
            ColumnBatch::Varchar(VarcharColumnBatch {
                name: "s3".to_string(),
                values: vec!["".to_string(), "".to_string()],
            }),
        ],
        num_rows: 2,
    };

    // Simulate S1 + S2 -> S3
    for row in 0..batch.num_rows {
        if let (Some(CellValue::Varchar(a)), Some(CellValue::Varchar(b))) =
            (batch.get(0, row), batch.get(1, row))
        {
            let concatenated = format!("{} {}", a, b);
            batch.set(2, row, CellValue::Varchar(concatenated)).unwrap();
        }
    }

    if let ColumnBatch::Varchar(col) = &batch.columns[2] {
        assert_eq!(col.values, vec!["Hello World".to_string(), "Foo Bar".to_string()]);
    } else {
        panic!("Expected Varchar column");
    }
}

#[test]
fn test_batch_set_boolean() {
    let mut batch = Batch {
        columns: vec![
            ColumnBatch::Boolean(BooleanColumnBatch {
                name: "f".to_string(),
                values: vec![true, false, true],
            }),
        ],
        num_rows: 3,
    };

    batch.set(0, 1, CellValue::Boolean(true)).unwrap();
    if let ColumnBatch::Boolean(col) = &batch.columns[0] {
        assert_eq!(col.values, vec![true, true, true]);
    } else {
        panic!("Expected Boolean column");
    }
}

#[test]
fn test_batch_set_type_mismatch() {
    let mut batch = Batch {
        columns: vec![
            ColumnBatch::Int64(Int64ColumnBatch {
                name: "v".to_string(),
                values: vec![10, 20, 30],
            }),
        ],
        num_rows: 3,
    };

    // Setting a Varchar value to an INT64 column should fail.
    let result = batch.set(0, 0, CellValue::Varchar("hello".to_string()));
    assert!(result.is_err());
}
