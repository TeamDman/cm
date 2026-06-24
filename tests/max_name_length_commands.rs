use cm::cli::Cli;

#[test]
fn max_name_length_show_parses() {
    assert!(figue::from_slice::<Cli>(&["max-name-length", "show"]).is_ok());
}

#[test]
fn max_name_length_set_parses() {
    assert!(figue::from_slice::<Cli>(&["max-name-length", "set", "50"]).is_ok());
}

#[test]
fn max_name_length_reset_parses() {
    assert!(figue::from_slice::<Cli>(&["max-name-length", "reset"]).is_ok());
}
