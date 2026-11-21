use super::decode_step_bytes;

#[test]
fn ascii_and_valid_utf8_are_preserved_byte_for_byte_as_text() {
    let ascii = b"FILE_NAME('plain ASCII','date');";
    assert_eq!(
        decode_step_bytes(ascii),
        std::str::from_utf8(ascii).unwrap()
    );

    let utf8 = "FILE_NAME('Caf\u{00e9}','date');".as_bytes();
    assert!(std::str::from_utf8(utf8).is_ok());
    assert_eq!(decode_step_bytes(utf8), std::str::from_utf8(utf8).unwrap());
}

#[test]
fn invalid_utf8_uses_the_same_lossless_latin1_mapping_as_from_step_bytes() {
    let mut bytes = b"FILE_NAME('Caf','date');".to_vec();
    bytes.insert(b"FILE_NAME('Caf".len(), 0xe9);
    assert!(std::str::from_utf8(&bytes).is_err());
    let expected = "FILE_NAME('Caf\u{00e9}','date');";
    assert_eq!(decode_step_bytes(&bytes), expected);
    assert_ne!(decode_step_bytes(&bytes), String::from_utf8_lossy(&bytes));
}
