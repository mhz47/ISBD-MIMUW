use isbd_dbms::encoding::vle::{encode_varint, decode_varint};

#[test]
fn test_varint_zero() {
    let mut encoded = Vec::new();
    encode_varint(&mut encoded, 0);
    assert_eq!(encoded, vec![0u8]);
    let (val, size) = decode_varint(&encoded).unwrap();
    assert_eq!(val, 0);
    assert_eq!(size, 1);
}

#[test]
fn test_varint_positive() {
    let mut encoded = Vec::new();
    encode_varint(&mut encoded, 42);
    let (val, _) = decode_varint(&encoded).unwrap();
    assert_eq!(val, 42);
}

#[test]
fn test_varint_negative() {
    let mut encoded = Vec::new();
    encode_varint(&mut encoded, -42);
    let (val, _) = decode_varint(&encoded).unwrap();
    assert_eq!(val, -42);
}

#[test]
fn test_varint_max() {
    let mut encoded = Vec::new();
    encode_varint(&mut encoded, i64::MAX);
    let (val, _) = decode_varint(&encoded).unwrap();
    assert_eq!(val, i64::MAX);
}

#[test]
fn test_varint_min() {
    let mut encoded = Vec::new();
    encode_varint(&mut encoded, i64::MIN);
    let (val, _) = decode_varint(&encoded).unwrap();
    assert_eq!(val, i64::MIN);
}

#[test]
fn test_varint_sequence() {
    for i in -100..100 {
        let mut encoded = Vec::new();
        encode_varint(&mut encoded, i);
        let (val, _) = decode_varint(&encoded).unwrap();
        assert_eq!(val, i, "Failed for {}", i);
    }
}
