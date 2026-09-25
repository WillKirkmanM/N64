use super::*;
#[test]
fn atomic_replacement_preserves_original_when_destination_cannot_be_replaced() {
    let directory = std::env::temp_dir().join(format!("p64-atomic-save-{}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let path = directory.join("save");
    fs::write(&path, b"old").unwrap();
    atomic_write(&path, b"new").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"new");
    let blocked = directory.join("blocked");
    fs::create_dir(&blocked).unwrap();
    fs::write(blocked.join("original"), b"keep").unwrap();
    assert!(atomic_write(&blocked, b"replacement").is_err());
    assert_eq!(fs::read(blocked.join("original")).unwrap(), b"keep");
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 2);
    fs::remove_dir_all(directory).unwrap();
}
