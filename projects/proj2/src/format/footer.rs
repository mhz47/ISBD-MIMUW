pub const MAGIC_HEADER: &[u8] = b"ISBD1";
pub const MAGIC_FOOTER: &[u8] = b"1DBSI";

/// Size of the footer length field (u64 little-endian).
pub const FOOTER_LENGTH_SIZE: usize = 8;

/// Serialize the footer metadata to bytes.
/// The footer contains: num_rows, num_columns, and column schemas with page offsets,
/// compressed sizes, and row counts.
pub fn serialize_footer(footer: &super::FileFooter) -> Vec<u8> {
    let mut result = Vec::new();
    
    // Number of rows (8 bytes, little-endian).
    result.extend_from_slice(&footer.num_rows.to_le_bytes());
    
    // Number of columns (8 bytes, little-endian).
    result.extend_from_slice(&footer.num_columns.to_le_bytes());
    
    // Column schemas.
    for col in &footer.columns {
        // Name length (4 bytes) + name bytes.
        let name_bytes = col.name.as_bytes();
        result.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
        result.extend_from_slice(name_bytes);
        // Column type (1 byte: 0=Int64, 1=Boolean, 2=Varchar).
        let type_byte = match col.column_type {
            super::ColumnType::Int64 => 0u8,
            super::ColumnType::Boolean => 1u8,
            super::ColumnType::Varchar => 2u8,
        };
        result.push(type_byte);
        // Number of pages (4 bytes).
        result.extend_from_slice(&col.num_pages.to_le_bytes());
        // Page offsets.
        result.extend_from_slice(&(col.page_offsets.len() as u32).to_le_bytes());
        for offset in &col.page_offsets {
            result.extend_from_slice(&offset.to_le_bytes());
        }
        // Page compressed sizes.
        for size in &col.page_compressed_sizes {
            result.extend_from_slice(&size.to_le_bytes());
        }
        // Page row counts (4 bytes each, little-endian).
        for count in &col.page_row_counts {
            result.extend_from_slice(&count.to_le_bytes());
        }
    }
    result
}

/// Deserialize the footer metadata from bytes.
/// The footer contains: num_rows, num_columns, and column schemas with page offsets,
/// compressed sizes, and row counts.
pub fn deserialize_footer(data: &[u8]) -> Result<super::FileFooter, String> {
    // Validate footer size before any parsing to fail fast on malicious files.
    const MAX_FOOTER_SIZE: usize = 64 * 1024 * 1024; // 64 MB
    if data.len() > MAX_FOOTER_SIZE {
        return Err(format!(
            "Footer size {} bytes exceeds maximum {} bytes",
            data.len(), MAX_FOOTER_SIZE
        ));
    }

    let mut pos = 0;

    // Read num_rows.
    if pos + 8 > data.len() {
        return Err("File too short for num_rows".into());
    }
    let num_rows = u64::from_le_bytes(
        data[pos..pos+8].try_into().expect("num_rows is exactly 8 bytes")
    );
    pos += 8;

    // Read num_columns.
    if pos + 8 > data.len() {
        return Err("File too short for num_columns".into());
    }
    let num_columns = u64::from_le_bytes(
        data[pos..pos+8].try_into().expect("num_columns is exactly 8 bytes")
    );
    pos += 8;

    // Read column schemas.
    let mut columns = Vec::with_capacity(num_columns as usize);
    for _ in 0..num_columns {
        // Name length.
        if pos + 4 > data.len() {
            return Err("File too short for column name length".into());
        }
        let name_len = u32::from_le_bytes(
            data[pos..pos+4].try_into().expect("name_len is exactly 4 bytes")
        ) as usize;
        pos += 4;

        // Name bytes.
        if pos + name_len > data.len() {
            return Err("File too short for column name".into());
        }
        let name = String::from_utf8(data[pos..pos+name_len].into())
            .map_err(|e| format!("Invalid UTF-8 in column name: {}", e))?;
        pos += name_len;

        // Column type.
        if pos + 1 > data.len() {
            return Err("File too short for column type".into());
        }
        let column_type = match data[pos] {
            0 => super::ColumnType::Int64,
            1 => super::ColumnType::Boolean,
            2 => super::ColumnType::Varchar,
            other => return Err(format!("Invalid column type: {}", other)),
        };
        pos += 1;

        // Number of pages (4 bytes).
        if pos + 4 > data.len() {
            return Err("File too short for num_pages".into());
        }
        let num_pages = u32::from_le_bytes(
            data[pos..pos+4].try_into().expect("num_pages is exactly 4 bytes")
        );
        pos += 4;

        // Page offsets count + offsets.
        if pos + 4 > data.len() {
            return Err("File too short for page offsets count".into());
        }
        let num_offsets = u32::from_le_bytes(
            data[pos..pos+4].try_into().expect("offsets count is exactly 4 bytes")
        ) as usize;
        pos += 4;
        let mut page_offsets = Vec::with_capacity(num_offsets);
        for _ in 0..num_offsets {
            if pos + 8 > data.len() {
                return Err("File too short for page offset".into());
            }
            page_offsets.push(u64::from_le_bytes(
                data[pos..pos+8].try_into().expect("page offset is exactly 8 bytes")
            ));
            pos += 8;
        }

        // Page compressed sizes.
        let mut page_compressed_sizes = Vec::with_capacity(num_offsets);
        for _ in 0..num_offsets {
            if pos + 8 > data.len() {
                return Err("File too short for page compressed size".into());
            }
            page_compressed_sizes.push(u64::from_le_bytes(
                data[pos..pos+8].try_into().expect("page compressed size is exactly 8 bytes")
            ));
            pos += 8;
        }

        // Page row counts (4 bytes each).
        let mut page_row_counts = Vec::with_capacity(num_offsets);
        for _ in 0..num_offsets {
            if pos + 4 > data.len() {
                return Err("File too short for page row count".into());
            }
            page_row_counts.push(u32::from_le_bytes(
                data[pos..pos+4].try_into().expect("page row count is exactly 4 bytes")
            ));
            pos += 4;
        }

        // Validate that all page arrays have the same length as num_pages.
        if page_offsets.len() != num_pages as usize {
            return Err(format!("Column {}: page_offsets length {} != num_pages {}",
                name, page_offsets.len(), num_pages));
        }
        if page_compressed_sizes.len() != num_pages as usize {
            return Err(format!("Column {}: page_compressed_sizes length {} != num_pages {}",
                name, page_compressed_sizes.len(), num_pages));
        }
        if page_row_counts.len() != num_pages as usize {
            return Err(format!("Column {}: page_row_counts length {} != num_pages {}",
                name, page_row_counts.len(), num_pages));
        }

        columns.push(super::ColumnSchema {
            name,
            column_type,
            num_pages,
            page_offsets,
            page_compressed_sizes,
            page_row_counts,
        });
    }

    Ok(super::FileFooter {
        num_rows,
        num_columns,
        columns,
    })
}
