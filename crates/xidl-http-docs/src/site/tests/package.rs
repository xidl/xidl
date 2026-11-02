use std::collections::BTreeSet;

#[test]
fn crate_inventory_keeps_scaffold_without_installed_frontend_dependencies() {
    let source: toml::Value = toml::from_str(include_str!("../../../Cargo.toml")).unwrap();
    let fixture = tempfile::tempdir().unwrap();
    let manifest = serde_json::json!({
        "package": {
            "name": "documentation-package-fixture",
            "version": "0.0.0",
            "edition": "2024",
            "description": "Documentation package inventory fixture",
            "license": "Apache-2.0",
            "include": source["package"]["include"],
        },
        "workspace": {},
    });
    std::fs::write(
        fixture.path().join("Cargo.toml"),
        toml::to_string(&manifest).unwrap(),
    )
    .unwrap();
    let required = [
        "README.md",
        "src/lib.rs",
        "frontend/README.md",
        "frontend/package.json",
        "frontend/pnpm-lock.yaml",
        "frontend/pnpm-workspace.yaml",
        "frontend/astro.config.mjs",
        "frontend/tsconfig.json",
        "frontend/.gitignore",
        "frontend/src/content.config.ts",
        "frontend/public/favicon.svg",
        "frontend/Scalar.LICENSE.txt",
        "frontend/scripts/prepare-scalar.mjs",
        "frontend/src/shiki/idl.mjs",
        "frontend/src/route-data.ts",
        "frontend/src/components/Head.astro",
        "frontend/src/scalar-theme.ts",
    ];
    let ignored = [
        "frontend/node_modules/dependency/README.md",
        "frontend/node_modules/dependency/package.json",
        "frontend/node_modules/dependency/nested/README.md",
        "frontend/dist/README.md",
        "frontend/.astro/README.md",
        "frontend/public/_scalar/scalar.js",
        "frontend/public/_scalar/LICENSE.txt",
    ];
    for path in required.iter().chain(&ignored) {
        let file = fixture.path().join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, "").unwrap();
    }
    let output = std::process::Command::new(env!("CARGO"))
        .args(["package", "--list", "--offline", "--allow-dirty"])
        .current_dir(fixture.path())
        .env("CARGO_TARGET_DIR", fixture.path().join("target"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let files = stdout
        .lines()
        .map(|path| path.replace('\\', "/"))
        .collect::<BTreeSet<_>>();
    for path in required {
        assert!(files.contains(path), "missing packaged scaffold: {path}");
    }
    for path in ignored {
        assert!(
            !files.contains(path),
            "packaged frontend build file: {path}"
        );
    }
}
