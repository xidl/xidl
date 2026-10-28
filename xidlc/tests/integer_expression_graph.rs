#![cfg(all(feature = "gen-rust", feature = "gen-go"))]

use std::collections::HashMap;
use std::fs;
use std::process::Command;

#[test]
fn repeated_references_compile_without_expanding_the_generated_expression() {
    let root = tempfile::tempdir().unwrap();
    let mut source = String::from("const unsigned long X0 = 1;\n");
    for index in 1..=30 {
        source.push_str(&format!(
            "const unsigned long X{index} = X{} + X{};\n",
            index - 1,
            index - 1
        ));
    }
    for language in ["rust", "go"] {
        let directory = root.path().join(language);
        fs::create_dir(&directory).unwrap();
        let files = xidlc::driver::Generator::new(language.into())
            .generate_from_idl(&source, &directory.join("limits.idl"), HashMap::new())
            .expect("repeated references form a shared integer expression graph");
        assert_eq!(files.len(), 1);
        let file = &files[0];
        assert!(
            file.content().len() < 250_000,
            "thirty levels must not inline a billion leaves"
        );
        fs::write(directory.join(file.path()), file.content()).unwrap();
        let mut command = if language == "rust" {
            fs::write(
                directory.join("main.rs"),
                r#"
                mod limits;
                fn main() {
                    let value = limits::X30;
                    let _: &u32 = &value;
                    assert_eq!(value, 1 << 30);
                }
            "#,
            )
            .unwrap();
            let mut command =
                Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()));
            command.args(["--edition=2024", "main.rs", "-o", "verify"]);
            command
        } else {
            fs::write(
                directory.join("limits_test.go"),
                r#"
                package limits
                import "testing"
                var inferred = X30
                var _ *uint32 = &inferred
                func TestConstant(t *testing.T) {
                    if X30 != 1 << 30 { t.Fatal(X30) }
                }
            "#,
            )
            .unwrap();
            let mut command = Command::new("go");
            command.args(["test", "limits.go", "limits_test.go"]);
            command.envs([
                ("GOPROXY", "off"),
                ("GOSUMDB", "off"),
                ("GOTOOLCHAIN", "local"),
                ("GOWORK", "off"),
            ]);
            command
        };
        let compiled = command.current_dir(&directory).output().unwrap();
        assert!(
            compiled.status.success(),
            "{language}: {}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        if language == "rust" {
            assert!(
                Command::new(directory.join("verify"))
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        }
    }
}
