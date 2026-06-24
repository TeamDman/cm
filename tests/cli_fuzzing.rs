use cm::cli::Cli;

#[test]
fn empty_cli_args_leave_command_unselected() {
    let cli: Cli = figue::Driver::new(
        figue::builder::<Cli>()
            .expect("schema should be valid")
            .cli(|cli| cli.args([] as [&str; 0]).strict())
            .build(),
    )
    .run()
    .unwrap();

    assert!(cli.command.is_none());
}

#[test]
fn fuzz_cli_args_consistency() {
    figue::assert_to_args_consistency::<Cli>(Default::default()).unwrap();
}

#[test]
fn fuzz_cli_args_roundtrip() {
    figue::assert_to_args_roundtrip::<Cli>(Default::default()).unwrap();
    
}
