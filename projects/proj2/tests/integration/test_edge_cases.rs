use isbd_dbms::format::{Batch, ColumnBatch, Int64ColumnBatch, BooleanColumnBatch, VarcharColumnBatch};
use super::utils::{deserialize_file, serialize_batches};

#[test]
fn test_roundtrip_mixed_strings() {
    let batch = Batch {
        columns: vec![
            ColumnBatch::Varchar(VarcharColumnBatch {
                name: "mixed".to_string(),
                values: vec![
                    "".to_string(),
                    "a".to_string(),
                    " ".to_string(),
                    "hello\nworld".to_string(),
                    "tab\there".to_string(),
                ],
            }),
        ],
        num_rows: 5,
    };

    let batches = vec![batch];
    let path = "/tmp/test_roundtrip_mixed.isbd";
    serialize_batches(batches, path).expect("Serialization failed");

    let deserialized = deserialize_file(path).expect("Deserialization failed");

    assert_eq!(deserialized.len(), 1);
    match &deserialized[0].columns[0] {
        ColumnBatch::Varchar(col) => {
            assert_eq!(col.values[0], "");
            assert_eq!(col.values[1], "a");
            assert_eq!(col.values[2], " ");
            assert_eq!(col.values[3], "hello\nworld");
            assert_eq!(col.values[4], "tab\there");
        }
        _ => panic!("Expected Varchar column"),
    }

    std::fs::remove_file(path).ok();
}

#[test]
fn test_roundtrip_int64_extremes() {
    let batch = Batch {
        columns: vec![
            ColumnBatch::Int64(Int64ColumnBatch {
                name: "extremes".to_string(),
                values: vec![i64::MAX, i64::MIN, 0, -1, 1],
            }),
        ],
        num_rows: 5,
    };

    let batches = vec![batch];
    let path = "/tmp/test_roundtrip_extremes.isbd";
    serialize_batches(batches, path).expect("Serialization failed");

    let deserialized = deserialize_file(path).expect("Deserialization failed");

    assert_eq!(deserialized.len(), 1);
    match &deserialized[0].columns[0] {
        ColumnBatch::Int64(col) => {
            assert_eq!(col.values, vec![i64::MAX, i64::MIN, 0, -1, 1]);
        }
        _ => panic!("Expected Int64 column"),
    }

    std::fs::remove_file(path).ok();
}

#[test]
fn test_roundtrip_empty_table() {
    let batch = Batch {
        columns: vec![
            ColumnBatch::Int64(Int64ColumnBatch {
                name: "id".to_string(),
                values: vec![],
            }),
            ColumnBatch::Boolean(BooleanColumnBatch {
                name: "active".to_string(),
                values: vec![],
            }),
            ColumnBatch::Varchar(VarcharColumnBatch {
                name: "text".to_string(),
                values: vec![],
            }),
        ],
        num_rows: 0,
    };

    let batches = vec![batch];
    let path = "/tmp/test_roundtrip_empty.isbd";
    serialize_batches(batches, path).expect("Serialization failed");

    let deserialized = deserialize_file(path).expect("Deserialization failed");

    assert_eq!(deserialized.len(), 0);

    std::fs::remove_file(path).ok();
}
