use isbd_dbms::format::{Batch, deserializer::BatchIterator, serializer::serialize_to_file};

/// Deserialize all batches from a file path into a Vec.
/// WARNING: This loads all batches into memory.
pub fn deserialize_file(path: &str) -> Result<Vec<Batch>, String> {
    let iter = BatchIterator::new(path)?;
    let mut batches = Vec::new();
    for batch_result in iter {
        batches.push(batch_result?);
    }
    Ok(batches)
}

/// Serialize a vector of batches to a file.
pub fn serialize_batches(batches: Vec<Batch>, path: &str) -> Result<(), String> {
    let iter = batches.into_iter().map(Ok);
    serialize_to_file(iter, path)
}
