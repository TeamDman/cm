use clap::Parser;
use cm::cli::Cli;

#[test]
fn site_show_parses() {
    assert!(Cli::try_parse_from(&["cm", "site", "show"]).is_ok());
}

#[test]
fn site_set_parses() {
    assert!(Cli::try_parse_from(&["cm", "site", "set", "my-site-id"]).is_ok());
}

#[test]
fn site_reset_parses() {
    assert!(Cli::try_parse_from(&["cm", "site", "reset"]).is_ok());
}

#[test]
fn search_parses() {
    assert!(Cli::try_parse_from(&["cm", "search", "hello"]).is_ok());
}
