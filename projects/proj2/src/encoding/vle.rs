/// Encode an i64 value using ZigZag + Variable Length Encoding, appending directly to the provided buffer.
pub fn encode_varint(buf: &mut Vec<u8>, value: i64) {
    let zigzag = (value << 1) ^ (value >> 63);
    encode_varint_u64(buf, zigzag as u64);
}

fn encode_varint_u64(buf: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80; // Set continuation bit.
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}

/// Decode a varint-encoded value from bytes, returning (value, bytes_consumed).
pub fn decode_varint(data: &[u8]) -> Result<(i64, usize), String> {
    let (uval, size) = decode_varint_u64(data)?;
    // Reverse ZigZag.
    let value = (uval >> 1) as i64 ^ (-((uval & 1) as i64));
    Ok((value, size))
}

fn decode_varint_u64(data: &[u8]) -> Result<(u64, usize), String> {
    let mut result: u64 = 0;
    let mut shift = 0;

    for (i, &byte) in data.iter().enumerate() {
        let seven_bits = (byte & 0x7F) as u64;
        result |= seven_bits << shift;

        if byte & 0x80 == 0 {
            // No more continuation.
            return Ok((result, i + 1));
        }
        
        shift += 7;
        if shift >= 64 {
            return Err("Varint too long".into());
        }
    }

    Err("Incomplete varint".into())
}
