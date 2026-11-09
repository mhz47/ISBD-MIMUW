use super::vle::{encode_varint, decode_varint};

/// Encode a VARCHAR page: for each string, write length (varint) + UTF-8 bytes, then compress with ZSTD.
pub fn encode_varchar_page(values: &[String]) -> Result<Vec<u8>, String> {
    if values.is_empty() {
        return Ok(Vec::new());
    }
    
    // Build raw data: length (varint) + bytes for each string.
    // Estimate capacity: average string length + 2 bytes per varint length prefix.
    let estimated_size: usize = values.iter().map(|s| s.len() + 2).sum();
    let mut raw = Vec::with_capacity(estimated_size);
    for s in values {
        let bytes = s.as_bytes();
        encode_varint(&mut raw, bytes.len() as i64);
        raw.extend_from_slice(bytes);
    }
    
    // Compress with ZSTD.
    compress(&raw)
}

/// Decode a VARCHAR page: decompress ZSTD, then parse length (varint) + bytes for each string.
pub fn decode_varchar_page(data: &[u8]) -> Result<Vec<String>, String> {
    if data.is_empty() {
        return Ok(Vec::new());
    }
    
    // Decompress ZSTD.
    let raw = decompress(data)?;
    
    // Parse strings.
    // Each string has at least a 1-byte varint length prefix, so max strings = raw.len().
    // Cap at a reasonable estimate to avoid allocating too much on corrupted data.
    let mut result = Vec::with_capacity(std::cmp::min(raw.len(), 4096));
    let mut pos = 0;
    while pos < raw.len() {
        let (len, size) = decode_varint(&raw[pos..])?;
        if len < 0 {
            return Err(format!("Invalid string length: {}", len));
        }
        pos += size;
        let len = len as usize;
        
        if pos + len > raw.len() {
            return Err("Truncated string data".into());
        }
        
        let s = String::from_utf8(raw[pos..pos+len].into())
            .map_err(|e| format!("Invalid UTF-8: {}", e))?;
        result.push(s);
        pos += len;
    }
    
    Ok(result)
}

/// Compress data using ZSTD.
fn compress(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.is_empty() {
        return Ok(Vec::new());
    }
    
    zstd::encode_all(data, 3).map_err(|e| format!("ZSTD compression failed: {}", e))
}

/// Decompress ZSTD data.
fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.is_empty() {
        return Ok(Vec::new());
    }
    
    zstd::decode_all(data).map_err(|e| format!("ZSTD decompression failed: {}", e))
}

