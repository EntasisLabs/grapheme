//! Binary-level checks for flags that must not be read as source paths.

use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const TWO_UI_VIEWS: &str = r#"
state S { n: Int = 0 }
mutation Inc on S { apply { n: $n + 1 } }
view AlphaView on S { Text("{$n}") }
view BetaView on S { Text("{$n}") }
glyph Alpha on S { AlphaView }
glyph Beta on S { BetaView }
"#;

fn grapheme() -> Command {
    Command::new(env!("CARGO_BIN_EXE_grapheme"))
}

#[test]
fn version_flags_print_the_cli_version_and_exit_zero() {
    for flag in ["--version", "-V"] {
        let output = grapheme().arg(flag).output().expect("spawn grapheme");
        assert!(
            output.status.success(),
            "{flag} exited {:?}\nstderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
        assert!(
            stdout.contains("grapheme-cli") && stdout.contains(env!("CARGO_PKG_VERSION")),
            "{flag} stdout: {stdout}"
        );
        let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
        assert!(
            !stderr.contains("error:"),
            "{flag} was treated as a file name: {stderr}"
        );
    }
}

#[test]
fn help_flags_print_usage_and_exit_zero() {
    for flag in ["--help", "-h", "help"] {
        let output = grapheme().arg(flag).output().expect("spawn grapheme");
        assert!(
            output.status.success(),
            "{flag} exited {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
        assert!(stderr.contains("usage:"), "{flag} stderr: {stderr}");
        assert!(
            stderr.contains("--glyph"),
            "usage should document --glyph: {stderr}"
        );
    }
}

#[test]
fn run_without_glyph_lists_every_view_and_exits_nonzero() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("grapheme-cli-it-multi-view-{unique}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("multi.gr");
    fs::write(&path, TWO_UI_VIEWS).expect("write multi view file");

    let output = grapheme()
        .arg("run")
        .arg(&path)
        .output()
        .expect("spawn grapheme");
    let unknown = grapheme()
        .args(["run", path.to_str().expect("utf8 path"), "--glyph", "Nope"])
        .output()
        .expect("spawn grapheme");
    let _ = fs::remove_dir_all(&dir);

    assert!(
        !output.status.success(),
        "several views without --glyph should fail"
    );
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("Alpha") && stderr.contains("Beta"),
        "{stderr}"
    );
    assert!(stderr.contains("--glyph"), "{stderr}");

    assert!(!unknown.status.success(), "unknown --glyph should fail");
    let stderr = String::from_utf8(unknown.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("Nope") && stderr.contains("Alpha") && stderr.contains("Beta"),
        "{stderr}"
    );
}
