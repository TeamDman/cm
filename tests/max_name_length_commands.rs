use clap::Parser;
use cm::cli::Cli;

#[test]
fn max_name_length_show_parses() {
    assert!(Cli::try_parse_from(&["cm", "max-name-length", "show"]).is_ok());
}

#[test]
fn max_name_length_set_parses() {
    assert!(Cli::try_parse_from(&["cm", "max-name-length", "set", "50"]).is_ok());
}

#[test]
fn max_name_length_reset_parses() {
    assert!(Cli::try_parse_from(&["cm", "max-name-length", "reset"]).is_ok());
}
