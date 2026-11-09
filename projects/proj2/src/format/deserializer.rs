use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use super::{Batch, ColumnBatch, Int64ColumnBatch, BooleanColumnBatch, VarcharColumnBatch, ColumnType};
use super::footer::{MAGIC_HEADER, MAGIC_FOOTER, FOOTER_LENGTH_SIZE, deserialize_footer};
use crate::encoding;

/// Streaming iterator over pages from an ISBD file.
/// Each page corresponds to one `next()` call and returns a `Batch`.
pub struct BatchIterator {
    file: File,
    footer: super::FileFooter,
    current_page_idx: usize,
}

impl BatchIterator {
    pub fn new(path: &str) -> Result<Self, String> {
        let mut file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        
        // Get file size.
        let file_size = file.metadata().map_err(|e| format!("Failed to get file metadata: {}", e))?
            .len() as usize;
        
        // Check minimum size: MAGIC_HEADER + footer_length + footer + MAGIC_FOOTER.
        let min_size = MAGIC_HEADER.len() + FOOTER_LENGTH_SIZE + MAGIC_FOOTER.len();
        if file_size < min_size {
            return Err("File too short".into());
        }
        
        // Read and verify magic header.
        file.seek(SeekFrom::Start(0))
            .map_err(|e| format!("Failed to seek to start: {}", e))?;
        let mut magic_buf = [0u8; MAGIC_HEADER.len()];
        file.read_exact(&mut magic_buf)
            .map_err(|e| format!("Failed to read magic header: {}", e))?;
        if magic_buf.as_slice() != MAGIC_HEADER {
            return Err(format!("Invalid magic header: expected {:?}, got {:?}", MAGIC_HEADER, &magic_buf));
        }
        
        // Read and verify magic footer.
        file.seek(SeekFrom::End(-(MAGIC_FOOTER.len() as i64)))
            .map_err(|e| format!("Failed to seek to footer: {}", e))?;
        let mut footer_magic_buf = [0u8; MAGIC_FOOTER.len()];
        file.read_exact(&mut footer_magic_buf)
            .map_err(|e| format!("Failed to read magic footer: {}", e))?;
        if footer_magic_buf.as_slice() != MAGIC_FOOTER {
            return Err(format!("Invalid magic footer: expected {:?}, got {:?}", MAGIC_FOOTER, &footer_magic_buf));
        }
        
        // Read footer length.
        let footer_length_pos = file_size - MAGIC_FOOTER.len() - FOOTER_LENGTH_SIZE;
        file.seek(SeekFrom::Start(footer_length_pos as u64))
            .map_err(|e| format!("Failed to seek to footer length: {}", e))?;
        let mut footer_len_buf = [0u8; FOOTER_LENGTH_SIZE];
        file.read_exact(&mut footer_len_buf)
            .map_err(|e| format!("Failed to read footer length: {}", e))?;
        let footer_length = u64::from_le_bytes(footer_len_buf) as usize;
        
        // Read footer metadata.
        let footer_data_pos = footer_length_pos - footer_length;
        file.seek(SeekFrom::Start(footer_data_pos as u64))
            .map_err(|e| format!("Failed to seek to footer data: {}", e))?;
        let mut footer_data = vec![0u8; footer_length];
        file.read_exact(&mut footer_data)
            .map_err(|e| format!("Failed to read footer data: {}", e))?;
        
        let footer = deserialize_footer(&footer_data)?;
        
        Ok(BatchIterator {
            file,
            footer,
            current_page_idx: 0,
        })
    }
}

impl Iterator for BatchIterator {
    type Item = Result<Batch, String>;
    
    fn next(&mut self) -> Option<Self::Item> {
        // Iterate over pages: each page stores its own row count in the footer.
        let page_idx = self.current_page_idx;
    
        // Check if there are more pages to read.
        // All columns should have the same number of pages; use the first column as a reference.
        let max_pages = self.footer.columns.first()
            .map(|col| col.page_offsets.len())
            .unwrap_or(0);
        if page_idx >= max_pages {
            return None;
        }
        
        // Get the row count for this page from the first column's schema.
        let page_row_count = self.footer.columns.first()
            .and_then(|col| col.page_row_counts.get(page_idx))
            .copied()
            .unwrap_or(0) as usize;
        
        let mut columns = Vec::new();
        
        for col_schema in &self.footer.columns {
            // Seek to the page offset and decode the page data.
            let col_data = match read_page(&mut self.file, col_schema, page_idx, page_row_count) {
                Ok(d) => d,
                Err(e) => return Some(Err(e)),
            };
            columns.push(col_data);
        }
        
        self.current_page_idx += 1;
        
        Some(Ok(Batch {
            columns,
            num_rows: page_row_count,
        }))
    }
}

/// Read and decode a single page from a column.
fn read_page(
    file: &mut File,
    col_schema: &super::ColumnSchema,
    page_idx: usize,
    page_row_count: usize,
) -> Result<ColumnBatch, String> {
    if page_idx >= col_schema.page_offsets.len() {
        return Err(format!("Page index {} out of bounds for column {} ({} pages)", 
            page_idx, col_schema.name, col_schema.page_offsets.len()));
    }
    
    let offset = col_schema.page_offsets[page_idx] as usize;
    let size = col_schema.page_compressed_sizes[page_idx] as usize;
    
    let page_data = read_bytes_at(file, offset, size)?;
    
    match col_schema.column_type {
        ColumnType::Int64 => {
            let values = encoding::decode_int64_page(&page_data)?;
            Ok(ColumnBatch::Int64(Int64ColumnBatch {
                name: col_schema.name.clone(),
                values,
            }))
        }
        ColumnType::Boolean => {
            // page_row_count tells us exactly how many bits to unpack.
            let values = encoding::unpack_bits(&page_data, page_row_count)?;
            Ok(ColumnBatch::Boolean(BooleanColumnBatch {
                name: col_schema.name.clone(),
                values,
            }))
        }
        ColumnType::Varchar => {
            let values = encoding::decode_varchar_page(&page_data)?;
            Ok(ColumnBatch::Varchar(VarcharColumnBatch {
                name: col_schema.name.clone(),
                values,
            }))
        }
    }
}

/// Read a specific number of bytes from the file at a given offset.
fn read_bytes_at(file: &mut File, offset: usize, len: usize) -> Result<Vec<u8>, String> {
    if len == 0 {
        return Ok(Vec::new());
    }
    
    file.seek(SeekFrom::Start(offset as u64))
        .map_err(|e| format!("Failed to seek to offset {}: {}", offset, e))?;
    
    let mut buf = vec![0u8; len];
    file.read_exact(&mut buf)
        .map_err(|e| format!("Failed to read {} bytes at offset {}: {}", len, offset, e))?;
    
    Ok(buf)
}
