use super::vle::{encode_varint, decode_varint};

/// Encode an INT64 page: delta encoding + VLE within the page.
/// Delta is reset at the start of each page - the first row stores its full value.
pub fn encode_int64_page(values: &[i64]) -> Vec<u8> {
    if values.is_empty() {
        return Vec::new();
    }
    
    let delta_encoded = encode_delta(values);
    // Each i64 encodes to 1–10 bytes as a varint; estimate 5 bytes per value.
    let mut result = Vec::with_capacity(delta_encoded.len() * 5);
    for &val in &delta_encoded {
        encode_varint(&mut result, val);
    }
    result
}

/// Decode an INT64 page: VLE decoding + delta decoding.
pub fn decode_int64_page(data: &[u8]) -> Result<Vec<i64>, String> {
    if data.is_empty() {
        return Ok(Vec::new());
    }
    
    // Each varint is at least 1 byte, so the maximum number of values is data.len().
    let mut delta_values = Vec::with_capacity(data.len());
    let mut pos = 0;
    while pos < data.len() {
        let (val, size) = decode_varint(&data[pos..])?;
        delta_values.push(val);
        pos += size;
    }
    
    Ok(decode_delta(&delta_values))
}

/// Apply delta encoding to a sequence of i64 values.
/// encoded[0] = values[0]
/// encoded[i] = values[i] - values[i-1] for i > 0
pub fn encode_delta(values: &[i64]) -> Vec<i64> {
    if values.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(values.len());
    result.push(values[0]);

    for i in 1..values.len() {
        result.push(values[i].wrapping_sub(values[i - 1]));
    }

    result
}

/// Reverse delta encoding.
/// values[0] = encoded[0]
/// values[i] = values[i-1] + encoded[i] for i > 0
pub fn decode_delta(encoded: &[i64]) -> Vec<i64> {
    if encoded.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(encoded.len());
    result.push(encoded[0]);

    for i in 1..encoded.len() {
        result.push(result[i - 1].wrapping_add(encoded[i]));
    }

    result
}


