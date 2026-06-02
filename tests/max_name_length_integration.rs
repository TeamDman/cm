use cm::MaxNameLength;
use cm::app_home::AppHome;
use std::sync::atomic::Ordering;
use tempfile::tempdir;

#[test]
fn set_updates_in_memory_static() {
    let temp = tempdir().expect("temp dir");
    let app_home = AppHome(temp.path().to_path_buf());

    // Pick a non-default value to ensure change
    let val = 123_usize;
    MaxNameLength::set_to(&app_home, val).expect("set_to should succeed");

    assert_eq!(cm::MAX_NAME_LENGTH.load(Ordering::SeqCst), val);
}
