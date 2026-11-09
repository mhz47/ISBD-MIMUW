use isbd_dbms::encoding::boolean::{pack_bits, unpack_bits};

#[test]
fn test_pack_empty() {
    let values: Vec<bool> = vec![];
    let packed = pack_bits(&values);
    assert!(packed.is_empty());
    let unpacked = unpack_bits(&packed, 0).unwrap();
    assert!(unpacked.is_empty());
}

#[test]
fn test_pack_single_true() {
    let values = vec![true];
    let packed = pack_bits(&values);
    assert_eq!(packed, vec![0b00000001]);
    let unpacked = unpack_bits(&packed, 1).unwrap();
    assert_eq!(unpacked, values);
}

#[test]
fn test_pack_single_false() {
    let values = vec![false];
    let packed = pack_bits(&values);
    assert_eq!(packed, vec![0b00000000]);
    let unpacked = unpack_bits(&packed, 1).unwrap();
    assert_eq!(unpacked, values);
}

#[test]
fn test_pack_eight_bits() {
    let values = vec![true, false, true, false, true, false, true, false];
    let packed = pack_bits(&values);
    assert_eq!(packed, vec![0b01010101]);
    let unpacked = unpack_bits(&packed, 8).unwrap();
    assert_eq!(unpacked, values);
}

#[test]
fn test_pack_padding() {
    // 10 values = 2 bytes, last byte has 6 padding bits
    let values = vec![true, true, true, true, true, true, true, true, true, true];
    let packed = pack_bits(&values);
    assert_eq!(packed.len(), 2);
    let unpacked = unpack_bits(&packed, 10).unwrap();
    assert_eq!(unpacked, values);
}

#[test]
fn test_pack_all_false() {
    let values = vec![false; 16];
    let packed = pack_bits(&values);
    assert_eq!(packed, vec![0x00, 0x00]);
    let unpacked = unpack_bits(&packed, 16).unwrap();
    assert_eq!(unpacked, values);
}

#[test]
fn test_pack_roundtrip() {
    let values = vec![true, false, true, true, false, false, true, false, true];
    let packed = pack_bits(&values);
    let unpacked = unpack_bits(&packed, values.len()).unwrap();
    assert_eq!(unpacked, values);
}
