use super::*;

#[test]
fn parses_gen_subcommand_with_lang_enum_and_files() {
    let cli = Cli::try_parse_from(["xidlc", "gen", "rust", "demo.idl"]).expect("parse cli");
    match cli.command {
        Command::Gen(args) => match args.lang {
            GenLang::Rust(lang) => {
                assert_eq!(lang.files, vec![PathBuf::from("demo.idl")]);
                assert!(!lang.client);
                assert!(!lang.server);
            }
            _ => panic!("expected rust lang"),
        },
        Command::Fmt(_) => panic!("expected gen command"),
        Command::Import(_) => panic!("expected gen command"),
    }
}

#[test]
fn keeps_rust_defaulting_to_server_only() {
    let cli = Cli::try_parse_from(["xidlc", "gen", "rust", "demo.idl"])
        .expect("rust command should parse");
    let Command::Gen(args) = cli.command else {
        panic!("expected gen command");
    };
    let args = args.into_driver_args().expect("args should convert");

    assert!(!args.client);
    assert!(args.server);
}

#[test]
fn parses_typescript_rest_client_server_flags() {
    let cli = Cli::try_parse_from([
        "xidlc",
        "gen",
        "-o",
        "out",
        "typescript-rest",
        "--client",
        "--server",
        "demo.idl",
    ])
    .expect("parse cli");

    let Command::Gen(args) = cli.command else {
        panic!("expected gen command");
    };
    let args = args.into_driver_args().expect("build driver args");

    assert_eq!(args.lang, "typescript-rest");
    assert_eq!(args.out_dir, "out");
    assert_eq!(args.files, vec![PathBuf::from("demo.idl")]);
    assert!(args.client);
    assert!(args.server);
}

#[test]
fn defaults_typescript_rest_to_client_only() {
    let cli = Cli::try_parse_from(["xidlc", "gen", "typescript-rest", "demo.idl"])
        .expect("typescript-rest command should parse");
    let Command::Gen(args) = cli.command else {
        panic!("expected gen command");
    };
    let args = args.into_driver_args().expect("args should convert");

    assert!(args.client);
    assert!(!args.server);
}

#[test]
fn supports_typescript_rest_server_only() {
    let cli = Cli::try_parse_from(["xidlc", "gen", "typescript-rest", "--server", "demo.idl"])
        .expect("typescript-rest command should parse");
    let Command::Gen(args) = cli.command else {
        panic!("expected gen command");
    };
    let args = args.into_driver_args().expect("args should convert");

    assert!(!args.client);
    assert!(args.server);
}
