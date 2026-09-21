use super::ExecutablePage;
#[test]
fn sealed_native_pages_reject_bulk_writes() {
    let mut page = ExecutablePage::new().unwrap();

    assert!(page.insert_unsealed(&[0xC3]).is_some());
    assert!(!page.sealed);
    assert!(page.seal());
    let used = page.used;

    assert!(page.insert_unsealed(&[0xC3]).is_none());
    assert_eq!(page.used, used);
    assert!(page.insert(&[0xC3]).is_some());
    assert!(page.sealed);
}
