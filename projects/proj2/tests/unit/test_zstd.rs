use isbd_dbms::encoding::varchar::{encode_varchar_page, decode_varchar_page};

#[test]
fn test_varchar_empty() {
    let values: Vec<String> = vec![];
    let encoded = encode_varchar_page(&values).unwrap();
    let decoded = decode_varchar_page(&encoded).unwrap();
    assert!(decoded.is_empty());
}

#[test]
fn test_varchar_simple() {
    let values = vec!["hello".to_string(), "world".to_string()];
    let encoded = encode_varchar_page(&values).unwrap();
    let decoded = decode_varchar_page(&encoded).unwrap();
    assert_eq!(decoded, values);
}

#[test]
fn test_varchar_empty_strings() {
    let values = vec!["".to_string(), "a".to_string(), "".to_string()];
    let encoded = encode_varchar_page(&values).unwrap();
    let decoded = decode_varchar_page(&encoded).unwrap();
    assert_eq!(decoded, values);
}

#[test]
fn test_varchar_long() {
    let values = vec!["a".repeat(10000), "b".repeat(5000)];
    let encoded = encode_varchar_page(&values).unwrap();
    let decoded = decode_varchar_page(&encoded).unwrap();
    assert_eq!(decoded, values);
}

#[test]
fn test_varchar_very_short_strings() {
    let values: Vec<String> = (0..256).map(|i| (i as u8 as char).to_string()).collect();
    let encoded = encode_varchar_page(&values).unwrap();
    let decoded = decode_varchar_page(&encoded).unwrap();
    assert_eq!(decoded, values);
}

#[test]
fn test_varchar_very_long_string() {
    let values = vec!["A".repeat(1_000_000)];
    let encoded = encode_varchar_page(&values).unwrap();
    let decoded = decode_varchar_page(&encoded).unwrap();
    assert_eq!(decoded, values);
}

#[test]
fn test_varchar_repetitive_strings() {
    let values: Vec<String> = vec!["hello".to_string(); 1000];
    let raw_size: usize = values.iter().map(|s| s.len()).sum();
    let encoded = encode_varchar_page(&values).unwrap();
    let decoded = decode_varchar_page(&encoded).unwrap();
    assert_eq!(decoded, values);
    assert!(encoded.len() < raw_size,
        "Compressed size ({}) should be smaller than raw size ({})",
        encoded.len(), raw_size);
}
