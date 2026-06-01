use cm::MaxNameLength;
use std::env;
use std::sync::atomic::Ordering;
use tempfile::tempdir;

#[test]
fn set_updates_in_memory_static() {
    let temp = tempdir().expect("temp dir");
    // This integration test has one test case, so set the config directory before APP_HOME is read.
    unsafe {
        env::set_var("CM_CONFIG_DIR", temp.path());
    }

    // Pick a non-default value to ensure change
    let val = 123_usize;
    MaxNameLength::set_to(val).expect("set_to should succeed");

    assert_eq!(cm::MAX_NAME_LENGTH.load(Ordering::SeqCst), val);
}
