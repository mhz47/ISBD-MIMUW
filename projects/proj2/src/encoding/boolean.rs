/// Pack a vector of bools into bytes, 1 bit per value (LSB first).
pub fn pack_bits(values: &[bool]) -> Vec<u8> {
    if values.is_empty() {
        return Vec::new();
    }
    
    let num_bytes = values.len().div_ceil(8);
    let mut result = vec![0u8; num_bytes];
    
    for (i, &val) in values.iter().enumerate() {
        let byte_idx = i / 8;
        let bit_idx = i % 8;
        if val {
            result[byte_idx] |= 1 << bit_idx;
        }
    }
    
    result
}

/// Unpack bits from bytes into bools, 1 bit per value (LSB first).
pub fn unpack_bits(data: &[u8], count: usize) -> Result<Vec<bool>, String> {
    let expected_bytes = count.div_ceil(8);
    if data.len() < expected_bytes {
        return Err(format!("Not enough bytes: expected {}, got {}", expected_bytes, data.len()));
    }
    
    let mut result = Vec::with_capacity(count);
    for i in 0..count {
        let byte_idx = i / 8;
        let bit_idx = i % 8;
        let val = (data[byte_idx] >> bit_idx) & 1 == 1;
        result.push(val);
    }
    
    Ok(result)
}

