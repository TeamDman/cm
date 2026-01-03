use arbitrary::Arbitrary;
use clap::Parser;
use cm::cli::Cli;
use cm::cli::to_args::ToArgs;

#[test]
fn fuzz_cli_args_roundtrip() {
    let mut data = vec![42u8; 1024];
    let mut rng = arbitrary::Unstructured::new(&data);

    for i in 0..100 {
        let cli = match Cli::arbitrary(&mut rng) {
            Ok(cli) => cli,
            Err(_) => {
                data = vec![i as u8; 1024];
                rng = arbitrary::Unstructured::new(&data);
                Cli::arbitrary(&mut rng).expect("Failed to generate CLI instance")
            }
        };

        let args = cli.to_args();

        let mut full_args = vec!["test-exe".into()];
        full_args.extend(args);

        let parsed_cli = match Cli::try_parse_from(&full_args) {
            Ok(parsed) => parsed,
            Err(e) => panic!(
                "Failed to parse CLI args on iteration {}: {}\nOriginal CLI: {:?}\nArgs: {:?}",
                i, e, cli, full_args
            ),
        };

        if cli != parsed_cli {
            panic!(
                "CLI roundtrip failed on iteration {}:\nOriginal: {:?}\nParsed: {:?}\nArgs: {:?}",
                i, cli, parsed_cli, full_args
            );
        }
    }
}
