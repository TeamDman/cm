use cm::MaxNameLength;
use std::sync::atomic::Ordering;

#[test]
fn set_updates_in_memory_static() {
    // Pick a non-default value to ensure change
    let val = 123_usize;
    MaxNameLength::set_to(val).expect("set_to should succeed");

    assert_eq!(cm::MAX_NAME_LENGTH.load(Ordering::SeqCst), val);
}
