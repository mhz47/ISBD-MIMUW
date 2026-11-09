use isbd_dbms::metrics::{calculate_entropy_from_counts, calculate_int64_average, calculate_metrics_stream, calculate_varchar_char_count};

#[test]
fn test_int64_average() {
    let values = vec![1i64, 2, 3, 4, 5];
    let avg = calculate_int64_average(&values);
    assert!((avg - 3.0).abs() < 1e-10);
}

#[test]
fn test_int64_average_empty() {
    let values: Vec<i64> = vec![];
    let avg = calculate_int64_average(&values);
    assert!((avg - 0.0).abs() < 1e-10);
}

#[test]
fn test_boolean_entropy_all_true() {
    let entropy = calculate_entropy_from_counts(3, 3);
    assert!((entropy - 0.0).abs() < 1e-10);
}

#[test]
fn test_boolean_entropy_all_false() {
    let entropy = calculate_entropy_from_counts(0, 3);
    assert!((entropy - 0.0).abs() < 1e-10);
}

#[test]
fn test_boolean_entropy_half() {
    // p=0.5, entropy should be 1.0 (maximum)
    let entropy = calculate_entropy_from_counts(2, 4);
    assert!((entropy - 1.0).abs() < 1e-10);
}

#[test]
fn test_boolean_entropy_75_percent() {
    // p=0.75, entropy ≈ 0.811
    let entropy = calculate_entropy_from_counts(3, 4);
    assert!((entropy - 0.811).abs() < 0.01);
}

#[test]
fn test_boolean_entropy_empty() {
    let entropy = calculate_entropy_from_counts(0, 0);
    assert!((entropy - 0.0).abs() < 1e-10);
}

#[test]
fn test_varchar_char_count() {
    let values = vec!["abc".to_string(), "de".to_string()];
    let count = calculate_varchar_char_count(&values);
    assert_eq!(count, 5);
}

#[test]
fn test_varchar_char_count_empty() {
    let values: Vec<String> = vec![];
    let count = calculate_varchar_char_count(&values);
    assert_eq!(count, 0);
}

#[test]
fn test_varchar_char_count_empty_strings() {
    let values = vec!["".to_string(), "".to_string()];
    let count = calculate_varchar_char_count(&values);
    assert_eq!(count, 0);
}

#[test]
fn test_streaming_metrics_error_propagation() {
    let iter = std::iter::once(Err::<isbd_dbms::format::Batch, String>(
        "simulated deserialization error".to_string(),
    ));
    let result = calculate_metrics_stream(iter);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("simulated deserialization error"));
}
