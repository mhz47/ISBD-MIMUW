use isbd_dbms::format::{Batch, ColumnBatch, Int64ColumnBatch, serializer::serialize_to_file, BatchIterator};

#[test]
fn test_corrupted_magic_header() {
    // Create a valid file, then corrupt the magic header.
    let batch = Batch {
        columns: vec![ColumnBatch::Int64(Int64ColumnBatch {
            name: "v".to_string(),
            values: vec![1, 2, 3],
        })],
        num_rows: 3,
    };
    let tmp = "/tmp/test_corrupted_header.isbd";
    let iter = std::iter::once(Ok(batch));
    serialize_to_file(iter, tmp).unwrap();

    // Corrupt magic header.
    let mut data = std::fs::read(tmp).unwrap();
    data[0] = b'X'; // Change 'I' to 'X'
    std::fs::write(tmp, &data).unwrap();

    let result = BatchIterator::new(tmp);
    assert!(result.is_err());
    match result {
        Err(e) => assert!(e.contains("Invalid magic header")),
        Ok(_) => panic!("Expected error"),
    }

    std::fs::remove_file(tmp).ok();
}

#[test]
fn test_truncated_file() {
    let batch = Batch {
        columns: vec![ColumnBatch::Int64(Int64ColumnBatch {
            name: "v".to_string(),
            values: vec![1, 2, 3],
        })],
        num_rows: 3,
    };
    let tmp = "/tmp/test_truncated.isbd";
    let iter = std::iter::once(Ok(batch));
    serialize_to_file(iter, tmp).unwrap();

    // Truncate the file (remove footer and some data).
    let data = std::fs::read(tmp).unwrap();
    std::fs::write(tmp, &data[..data.len() / 2]).unwrap();

    let result = BatchIterator::new(tmp);
    assert!(result.is_err());

    std::fs::remove_file(tmp).ok();
}

#[test]
fn test_empty_file() {
    let tmp = "/tmp/test_empty.isbd";
    std::fs::write(tmp, "").unwrap();

    let result = BatchIterator::new(tmp);
    assert!(result.is_err());

    std::fs::remove_file(tmp).ok();
}

#[test]
fn test_nonexistent_file() {
    let result = BatchIterator::new("/tmp/does_not_exist_12345.isbd");
    assert!(result.is_err());
}

#[test]
fn test_serialize_empty_batches() {
    let iter = std::iter::empty::<Result<Batch, String>>();
    let result = serialize_to_file(iter, "/tmp/test_empty_batches.isbd");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No batches"));
}
