use super::*;

#[test]
fn historical_and_map_only_saves_recover_identical_data() {
    let plain = br#"{"4096": 17}"#;
    let legacy = br#"[{"4096": 17}, true]"#;
    let data = decode_romsave(plain).unwrap();

    assert_eq!(data["4096"], 17);
    assert_eq!(decode_romsave(legacy).unwrap(), data);
}

#[test]
fn malformed_save_documents_are_rejected_without_partial_data() {
    for invalid in [b"{".as_slice(), b"null", b"[{}, 7]", b"[]"] {
        assert!(decode_romsave(invalid).is_err());
    }
}
