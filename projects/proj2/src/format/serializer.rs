use std::fs::File;
use std::io::{BufWriter, Write};
use super::{Batch, ColumnBatch, FileFooter, ColumnSchema, PAGE_SIZE};
use super::footer::{MAGIC_HEADER, MAGIC_FOOTER, serialize_footer};
use crate::encoding;

/// Streaming serializer: accepts an iterator of batches, writes pages to disk.
pub fn serialize_to_file<I>(batch_iter: I, path: &str) -> Result<(), String>
where
    I: Iterator<Item = Result<Batch, String>>,
{
    // Create temporary file.
    let tmp_path = format!("{}.tmp", path);
    let file = File::create(&tmp_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    let mut writer = BufWriter::new(file);
    let mut guard = TempFileGuard::new(tmp_path.clone());

    // Write magic header.
    writer.write_all(MAGIC_HEADER)
        .map_err(|e| format!("Failed to write magic header: {}", e))?;

    // Collect metadata and write pages batch by batch.
    // Each batch produces one or more pages per column (if batch > PAGE_SIZE, it splits).
    let mut column_schemas: Option<Vec<ColumnSchema>> = None;
    let mut total_rows: usize = 0;
    let mut batch_idx: usize = 0;
    let mut current_offset = MAGIC_HEADER.len() as u64;

    for batch_result in batch_iter {
        let batch = batch_result?;

        let schemas = if let Some(schemas) = &mut column_schemas {
            // Validate consistency with the first batch.
            if batch.columns.len() != schemas.len() {
                return Err(format!(
                    "Batch {} has {} columns, expected {}",
                    batch_idx, batch.columns.len(), schemas.len()
                ));
            }
            for (col_idx, (col, first_col)) in batch.columns.iter().zip(schemas.iter()).enumerate() {
                if col.column_type() != first_col.column_type {
                    return Err(format!(
                        "Batch {} column {} has type {:?}, expected {:?}",
                        batch_idx, col_idx, col.column_type(), first_col.column_type
                    ));
                }
            }
            schemas
        } else {
            // First batch - initialize column schemas.
            let num_columns = batch.columns.len();
            let mut new_schemas = Vec::with_capacity(num_columns);
            for col in &batch.columns {
                new_schemas.push(ColumnSchema {
                    name: col.name().to_string(),
                    column_type: col.column_type(),
                    num_pages: 0,
                    page_offsets: Vec::new(),
                    page_compressed_sizes: Vec::new(),
                    page_row_counts: Vec::new(),
                });
            }
            column_schemas = Some(new_schemas);
            column_schemas.as_mut().unwrap()
        };
        let num_rows = batch.num_rows;
        for (col_idx, col) in batch.columns.iter().enumerate() {
            let mut row_start = 0;
            while row_start < num_rows {
                let row_end = std::cmp::min(row_start + PAGE_SIZE, num_rows);

                let page_data = match col {
                    ColumnBatch::Int64(c) => {
                        encoding::encode_int64_page(&c.values[row_start..row_end])
                    }
                    ColumnBatch::Boolean(c) => {
                        encoding::pack_bits(&c.values[row_start..row_end])
                    }
                    ColumnBatch::Varchar(c) => {
                        encoding::encode_varchar_page(&c.values[row_start..row_end])
                            .map_err(|e| format!("Failed to encode VARCHAR page: {}", e))?
                    }
                };

                // Record offset, size, and row count for the footer.
                let page_row_count = row_end - row_start;
                schemas[col_idx].page_offsets.push(current_offset);
                schemas[col_idx].page_compressed_sizes.push(page_data.len() as u64);
                schemas[col_idx].page_row_counts.push(page_row_count as u32);
                schemas[col_idx].num_pages += 1;
                current_offset += page_data.len() as u64;

                // Write page data directly to file.
                writer.write_all(&page_data)
                    .map_err(|e| format!("Failed to write page data: {}", e))?;

                row_start = row_end;
            }
        }

        total_rows += batch.num_rows;
        batch_idx += 1;
    }
    
    if batch_idx == 0 {
        return Err("No batches to serialize".into());
    }

    // Flush page data before writing the footer.
    writer.flush()
        .map_err(|e| format!("Failed to flush page data: {}", e))?;

    // Build footer metadata with collected page offsets and sizes.
    let footer = FileFooter {
        num_rows: total_rows as u64,
        num_columns: column_schemas.as_ref().unwrap().len() as u64,
        columns: column_schemas.unwrap(),
    };
    let footer_bytes = serialize_footer(&footer);

    // Write footer, footer_length, and magic footer.
    writer.write_all(&footer_bytes)
        .map_err(|e| format!("Failed to write footer: {}", e))?;
    writer.write_all(&(footer_bytes.len() as u64).to_le_bytes())
        .map_err(|e| format!("Failed to write footer length: {}", e))?;
    writer.write_all(MAGIC_FOOTER)
        .map_err(|e| format!("Failed to write magic footer: {}", e))?;

    // Flush and close the writer.
    writer.flush()
        .map_err(|e| format!("Failed to flush: {}", e))?;
    drop(writer);

    // Disarm the guard so the temp file is not removed on drop.
    guard.disarm();

    // Atomically rename the temp file to the final path.
    std::fs::rename(&tmp_path, path)
        .map_err(|e| format!("Failed to rename {} to {}: {}", tmp_path, path, e))?;

    Ok(())
}


/// Guard that automatically removes the temporary file on drop if it's still active.
/// This ensures that a failed serialization doesn't leave a stale `.tmp` file on disk.
struct TempFileGuard {
    path: String,
    active: bool,
}

impl TempFileGuard {
    fn new(path: String) -> Self {
        Self { path, active: true }
    }

    fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.active {
            // Clean up the temporary file if serialization failed.
            let _ = std::fs::remove_file(&self.path);
        }
    }
}