use isbd_dbms::encoding::int64::{encode_delta, decode_delta};

#[test]
fn test_delta_empty() {
    let values: Vec<i64> = vec![];
    let encoded = encode_delta(&values);
    let decoded = decode_delta(&encoded);
    assert_eq!(values, decoded);
}

#[test]
fn test_delta_single() {
    let values = vec![42i64];
    let encoded = encode_delta(&values);
    assert_eq!(encoded, vec![42i64]);
    let decoded = decode_delta(&encoded);
    assert_eq!(values, decoded);
}

#[test]
fn test_delta_sequential() {
    let values = vec![1i64, 2, 3, 4, 5];
    let encoded = encode_delta(&values);
    assert_eq!(encoded, vec![1i64, 1, 1, 1, 1]);
    let decoded = decode_delta(&encoded);
    assert_eq!(values, decoded);
}

#[test]
fn test_delta_negative() {
    let values = vec![5i64, 3, 1, -1, -3];
    let encoded = encode_delta(&values);
    assert_eq!(encoded, vec![5i64, -2, -2, -2, -2]);
    let decoded = decode_delta(&encoded);
    assert_eq!(values, decoded);
}

#[test]
fn test_delta_roundtrip() {
    let values = vec![100i64, -50, 200, -100, 0];
    let encoded = encode_delta(&values);
    let decoded = decode_delta(&encoded);
    assert_eq!(values, decoded);
}
