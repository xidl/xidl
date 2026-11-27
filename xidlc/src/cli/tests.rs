use super::version::VersionCommand;
use super::*;

#[test]
fn version_diff_uses_three_positional_arguments_and_rejects_legacy_options() {
    let cli = Cli::try_parse_from(["xidlc", "version", "diff", "v1", "2026-07", "api.idl"])
        .expect("parse version diff");
    let Command::Version(args) = cli.command else {
        panic!("expected version command");
    };
    let VersionCommand::Diff(args) = args.command else {
        panic!("expected version diff command");
    };
    assert_eq!(args.from, "v1");
    assert_eq!(args.to, "2026-07");
    assert_eq!(args.idl, PathBuf::from("api.idl"));
    for (args, expected_kind) in [
        (
            vec![
                "xidlc", "version", "diff", "--from", "v1", "--to", "v2", "api.idl",
            ],
            clap::error::ErrorKind::UnknownArgument,
        ),
        (
            vec!["xidlc", "version", "diff", "v1", "v2"],
            clap::error::ErrorKind::MissingRequiredArgument,
        ),
        (
            vec!["xidlc", "version", "verify", "api.idl"],
            clap::error::ErrorKind::InvalidSubcommand,
        ),
    ] {
        let err = Cli::try_parse_from(args).expect_err("invalid version args must fail");
        assert_eq!(err.kind(), expected_kind);
    }
}

#[test]
fn version_lock_and_check_parse_idl_and_options() {
    let lock_default = Cli::try_parse_from(["xidlc", "version", "lock", "api.idl"])
        .expect("parse default version lock");
    let Command::Version(args) = lock_default.command else {
        panic!("expected version command");
    };
    let VersionCommand::Lock(args) = args.command else {
        panic!("expected version lock command");
    };
    assert_eq!(args.lock_file, None);
    assert!(!args.force);
    assert_eq!(args.idl, PathBuf::from("api.idl"));

    let check_default = Cli::try_parse_from(["xidlc", "version", "check", "api.idl"])
        .expect("parse default version check");
    let Command::Version(args) = check_default.command else {
        panic!("expected version command");
    };
    let VersionCommand::Check(args) = args.command else {
        panic!("expected version check command");
    };
    assert_eq!(args.lock_file, None);
    assert_eq!(args.idl, PathBuf::from("api.idl"));

    for command in ["lock", "check"] {
        assert_eq!(
            Cli::try_parse_from(["xidlc", "version", command, "--strict", "api.idl"])
                .expect_err("version selection replaces --strict")
                .kind(),
            clap::error::ErrorKind::UnknownArgument
        );
    }
    assert_eq!(
        Cli::try_parse_from(["xidlc", "version", "check", "--force", "api.idl"])
            .expect_err("check must reject --force")
            .kind(),
        clap::error::ErrorKind::UnknownArgument
    );

    let lock = Cli::try_parse_from([
        "xidlc",
        "version",
        "lock",
        "--lock-file",
        "custom.lock.json",
        "--force",
        "api.idl",
    ])
    .expect("parse version lock");
    let Command::Version(args) = lock.command else {
        panic!("expected version command");
    };
    let VersionCommand::Lock(args) = args.command else {
        panic!("expected version lock command");
    };
    assert_eq!(args.lock_file, Some(PathBuf::from("custom.lock.json")));
    assert!(args.force);
    assert_eq!(args.idl, PathBuf::from("api.idl"));

    let check = Cli::try_parse_from([
        "xidlc",
        "version",
        "check",
        "-l",
        "custom.lock.json",
        "--versions",
        r#"["2026-12-01"]"#,
        "api.idl",
    ])
    .expect("parse version check");
    let Command::Version(args) = check.command else {
        panic!("expected version command");
    };
    let VersionCommand::Check(args) = args.command else {
        panic!("expected version check command");
    };
    assert_eq!(args.lock_file, Some(PathBuf::from("custom.lock.json")));
    assert_eq!(args.idl, PathBuf::from("api.idl"));
}

#[test]
fn version_selection_accepts_a_json_array_of_version_values() {
    for command in ["lock", "check"] {
        for selection in ["[]", r#"["2026-12-01","2027-06-01"]"#] {
            Cli::try_parse_from([
                "xidlc",
                "version",
                command,
                "--versions",
                selection,
                "api.idl",
            ])
            .expect("parse version array");
        }
        for invalid in ["", "{}", "[1]", "2026-12-01"] {
            assert!(
                Cli::try_parse_from([
                    "xidlc",
                    "version",
                    command,
                    "--versions",
                    invalid,
                    "api.idl",
                ])
                .is_err()
            );
        }
    }
}

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
        Command::Fmt(_) | Command::Lint(_) => panic!("expected gen command"),
        Command::Import(_) => panic!("expected gen command"),
        Command::Version(_) => panic!("expected gen command"),
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

#[test]
fn parses_single_api_version_selection() {
    let cli = Cli::try_parse_from(["xidlc", "gen", "--api-version", "v2", "openapi", "demo.idl"])
        .expect("version flag should parse");
    let Command::Gen(args) = cli.command else {
        panic!("expected gen command");
    };
    let args = args.into_driver_args().expect("args should convert");

    assert_eq!(args.api_version.as_deref(), Some("v2"));
    assert!(args.api_versions.is_none());
}

#[test]
fn parses_multiple_api_version_selection() {
    let cli = Cli::try_parse_from([
        "xidlc",
        "gen",
        "--api-versions",
        "v1,v3",
        "rust",
        "demo.idl",
    ])
    .expect("version flag should parse");
    let Command::Gen(args) = cli.command else {
        panic!("expected gen command");
    };
    let args = args.into_driver_args().expect("args should convert");

    assert_eq!(args.api_versions.as_deref(), Some("v1,v3"));
    assert!(args.api_version.is_none());
}

#[test]
fn rejects_conflicting_api_version_selection() {
    let err = Cli::try_parse_from([
        "xidlc",
        "gen",
        "--api-version",
        "v1",
        "--api-versions",
        "v2",
        "rust",
        "demo.idl",
    ])
    .expect_err("conflicting flags should not parse");

    assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
}

#[test]
fn parses_openapi_version_selection() {
    let cli = Cli::try_parse_from(["xidlc", "gen", "openapi", "--version", "3.1", "demo.idl"])
        .expect("openapi version flag should parse");
    let Command::Gen(args) = cli.command else {
        panic!("expected gen command");
    };
    let args = args.into_driver_args().expect("args should convert");

    assert_eq!(args.lang, "openapi");
    assert_eq!(args.openapi_version.as_deref(), Some("3.1"));
}

#[test]
fn openapi_defaults_to_auto_version() {
    let cli = Cli::try_parse_from(["xidlc", "gen", "openapi", "demo.idl"])
        .expect("openapi command should parse");
    let Command::Gen(args) = cli.command else {
        panic!("expected gen command");
    };
    let args = args.into_driver_args().expect("args should convert");

    assert!(args.openapi_version.is_none());
}

#[test]
fn openapi_version_only_parses_for_the_openapi_target() {
    let err = Cli::try_parse_from(["xidlc", "gen", "rust", "--version", "3.1", "demo.idl"])
        .expect_err("`rust` must reject --version");
    assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);

    let err = Cli::try_parse_from(["xidlc", "gen", "openapi", "--version", "3.2", "demo.idl"])
        .expect_err("only auto and 3.1 are accepted");
    assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
}

#[test]
fn lint_parses_full_protocol_and_deny_flags() {
    let cli = Cli::try_parse_from([
        "xidlc",
        "lint",
        "--deny",
        "warnings",
        "--full",
        "--protocol",
        "http",
        "api.idl",
    ])
    .expect("lint command with flags should parse");
    assert!(matches!(cli.command, Command::Lint(_)));
}
