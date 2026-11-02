use super::{CompilationOutput, File};
use std::fs;

fn file(path: &str, content: &str) -> File {
    File {
        path: path.into(),
        content: content.into(),
    }
}

#[test]
fn output_rejects_invalid_plans_before_changing_existing_files() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().to_str().unwrap();
    let output = CompilationOutput::new(directory, "http-docs");
    output.write(vec![file("index.html", "published")]).unwrap();
    let manifest = root.path().join(".xidlc-687474702d646f6373-outputs.json");
    let original = fs::read(&manifest).unwrap();
    for files in [
        vec![file("index.html", "changed"), file("../outside", "escape")],
        vec![file("index.html", "first"), file("index.html", "second")],
        vec![file("index.html", "changed"), file("", "empty")],
        vec![file("new", "parent"), file("new/child", "child")],
        vec![file(".xidlc-687474702d646f6373-outputs.json", "overwrite")],
    ] {
        assert!(output.write(files).is_err());
        assert_eq!(
            fs::read_to_string(root.path().join("index.html")).unwrap(),
            "published"
        );
        assert_eq!(fs::read(&manifest).unwrap(), original);
        assert!(!root.path().join("new").exists());
    }
}

#[test]
fn new_paths_do_not_claim_existing_user_files_on_first_or_later_publications() {
    for initial in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let output = CompilationOutput::new(root.path().to_str().unwrap(), "docs");
        if !initial {
            output.write(vec![file("owned.md", "previous")]).unwrap();
        }
        fs::write(root.path().join("user.md"), "user content").unwrap();
        let error = output
            .write(vec![file("owned.md", "new"), file("user.md", "generated")])
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("would overwrite a user-owned file")
        );
        assert_eq!(
            fs::read_to_string(root.path().join("user.md")).unwrap(),
            "user content"
        );
        if initial {
            assert!(!root.path().join("owned.md").exists());
        } else {
            assert_eq!(
                fs::read_to_string(root.path().join("owned.md")).unwrap(),
                "previous"
            );
        }
    }
}

#[test]
fn late_file_conflicts_preserve_the_previous_publication() {
    let root = tempfile::tempdir().unwrap();
    let output = CompilationOutput::new(root.path().to_str().unwrap(), "docs");
    output
        .write(vec![
            file("a.html", "previous"),
            file("stale.md", "previous"),
        ])
        .unwrap();
    let manifest = root.path().join(".xidlc-646f6373-outputs.json");
    let original = fs::read(&manifest).unwrap();
    fs::create_dir(root.path().join("z.html")).unwrap();
    let error = output
        .write(vec![file("a.html", "changed"), file("z.html", "conflict")])
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("conflicts with an existing file or directory")
    );
    assert_eq!(
        fs::read_to_string(root.path().join("a.html")).unwrap(),
        "previous"
    );
    assert!(root.path().join("stale.md").exists());
    assert_eq!(fs::read(&manifest).unwrap(), original);
}

#[test]
fn untrusted_previous_manifest_cannot_remove_parent_files() {
    let root = tempfile::tempdir().unwrap();
    let site = root.path().join("site");
    fs::create_dir(&site).unwrap();
    fs::write(root.path().join("user.md"), "user").unwrap();
    let manifest = site.join(".xidlc-646f6373-outputs.json");
    fs::write(&manifest, r#"["../user.md"]"#).unwrap();
    let output = CompilationOutput::new(site.to_str().unwrap(), "docs");
    assert!(output.write(vec![file("index.html", "new")]).is_err());
    assert_eq!(
        fs::read_to_string(root.path().join("user.md")).unwrap(),
        "user"
    );
    assert!(!site.join("index.html").exists());
}

#[cfg(unix)]
#[test]
fn output_rejects_symlink_writes_cleanup_and_manifest_reads() {
    use std::os::unix::fs::symlink;
    for kind in ["parent", "leaf", "stale", "manifest"] {
        let root = tempfile::tempdir().unwrap();
        let site = root.path().join("site");
        let external = root.path().join("external");
        fs::create_dir(&site).unwrap();
        fs::create_dir(&external).unwrap();
        fs::write(external.join("user.md"), "outside user content").unwrap();
        let manifest = site.join(".xidlc-646f6373-outputs.json");
        let files = match kind {
            "parent" => {
                symlink(&external, site.join("api")).unwrap();
                vec![file("api/user.md", "overwrite")]
            }
            "leaf" => {
                symlink(external.join("user.md"), site.join("user.md")).unwrap();
                vec![file("user.md", "overwrite")]
            }
            "stale" => {
                symlink(&external, site.join("api")).unwrap();
                fs::write(&manifest, r#"["api/user.md"]"#).unwrap();
                vec![file("index.html", "new")]
            }
            "manifest" => {
                symlink(external.join("user.md"), &manifest).unwrap();
                vec![file("index.html", "new")]
            }
            _ => unreachable!(),
        };
        let error = CompilationOutput::new(site.to_str().unwrap(), "docs")
            .write(files)
            .unwrap_err();
        assert!(
            error.to_string().contains("symbolic link"),
            "{kind}: {error}"
        );
        assert_eq!(
            fs::read_to_string(external.join("user.md")).unwrap(),
            "outside user content"
        );
        assert!(!site.join("index.html").exists());
    }
}
