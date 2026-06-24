use cm::cli::Cli;

#[test]
fn fuzz_cli_args_consistency() {
    figue::assert_to_args_consistency::<Cli>(Default::default()).unwrap();
}

#[test]
fn fuzz_cli_args_roundtrip() {
    figue::assert_to_args_roundtrip::<Cli>(Default::default()).unwrap();
    
}
