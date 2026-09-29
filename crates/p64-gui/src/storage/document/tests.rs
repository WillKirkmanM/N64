use super::*;

#[test]
fn historical_and_map_only_saves_recover_identical_data() {
    let plain = br#"{"4096": 17}"#;
    let legacy = br#"[{"4096": 17}, true]"#;
    let data = decode_romsave(plain).unwrap();

    assert_eq!(data["4096"], 17);
    assert_eq!(decode_romsave(legacy).unwrap(), data);
}
