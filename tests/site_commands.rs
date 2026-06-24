use cm::cli::Cli;

#[test]
fn site_show_parses() {
    assert!(figue::from_slice::<Cli>(&["site", "show"]).is_ok());
}

#[test]
fn site_set_parses() {
    assert!(figue::from_slice::<Cli>(&["site", "set", "my-site-id"]).is_ok());
}

#[test]
fn site_reset_parses() {
    assert!(figue::from_slice::<Cli>(&["site", "reset"]).is_ok());
}

#[test]
fn search_parses() {
    assert!(figue::from_slice::<Cli>(&["search", "hello"]).is_ok());
}
