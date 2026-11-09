use isbd_dbms::encoding::{encode_int64_page, decode_int64_page};

#[test]
fn test_int64_roundtrip() {
    let values = vec![1i64, 2, 3, 4, 5];
    let encoded = encode_int64_page(&values);
    let decoded = decode_int64_page(&encoded).unwrap();
    assert_eq!(values, decoded);
}

#[test]
fn test_int64_negative() {
    let values = vec![-5i64, -3, -1, 0, 2];
    let encoded = encode_int64_page(&values);
    let decoded = decode_int64_page(&encoded).unwrap();
    assert_eq!(values, decoded);
}

#[test]
fn test_int64_empty() {
    let values: Vec<i64> = vec![];
    let encoded = encode_int64_page(&values);
    let decoded = decode_int64_page(&encoded).unwrap();
    assert_eq!(values, decoded);
}

#[test]
fn test_int64_single() {
    let values = vec![42i64];
    let encoded = encode_int64_page(&values);
    let decoded = decode_int64_page(&encoded).unwrap();
    assert_eq!(values, decoded);
}

#[test]
fn test_int64_large_values() {
    let values = vec![i64::MAX, i64::MIN, 0];
    let encoded = encode_int64_page(&values);
    let decoded = decode_int64_page(&encoded).unwrap();
    assert_eq!(values, decoded);
}
