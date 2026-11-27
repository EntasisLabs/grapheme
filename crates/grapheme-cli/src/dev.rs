//! `grapheme dev`: watch a `.gr` file and reload its window in place.
//!
//! Watching and reloading come from grapheme-ui's `desktop::dev_file`,
//! behind the `ui` feature. Without it, the command explains
//! how to reinstall. The headless compiler stays on Rust 1.94.

#[cfg(feature = "ui")]
use std::path::Path;

use grapheme_compiler::CompilerError;

#[cfg(not(feature = "ui"))]
const DEV_UI_MISSING: &str = "grapheme dev watches a file and reloads its window, but this grapheme was built without UI support. Reinstall with: cargo install grapheme-cli --features ui";

/// `grapheme dev [<file.gr>] [--glyph <name>]`.
pub fn dev_command(args: &[String]) -> Result<(), CompilerError> {
    let (path, glyph) = parse_dev_args(args)?;
    launch_dev(&path, glyph.as_deref())
}

fn parse_dev_args(args: &[String]) -> Result<(String, Option<String>), CompilerError> {
    let mut file = None;
    let mut glyph = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--glyph" => {
                let name = args.get(index + 1).ok_or_else(|| {
                    CompilerError::RuntimeError("--glyph requires a glyph name".to_string())
                })?;
                if name.starts_with('-') {
                    return Err(CompilerError::RuntimeError(
                        "--glyph requires a glyph name".to_string(),
                    ));
                }
                glyph = Some(name.clone());
                index += 2;
            }
            "--help" | "-h" => {
                return Err(CompilerError::RuntimeError(
                    "usage: grapheme dev [<file.gr>] [--glyph <name>]".to_string(),
                ));
            }
            other if other.starts_with('-') => {
                return Err(CompilerError::RuntimeError(format!(
                    "unknown flag '{other}'"
                )));
            }
            other => {
                if file.is_some() {
                    return Err(CompilerError::RuntimeError(format!(
                        "unexpected argument '{other}'"
                    )));
                }
                file = Some(other.to_string());
                index += 1;
            }
        }
    }
    let file = file.ok_or_else(|| {
        CompilerError::RuntimeError(
            "dev requires a file path (grapheme dev <file.gr> [--glyph <name>])".to_string(),
        )
    })?;
    Ok((file, glyph))
}

#[cfg(not(feature = "ui"))]
fn launch_dev(_path: &str, _glyph: Option<&str>) -> Result<(), CompilerError> {
    Err(CompilerError::RuntimeError(DEV_UI_MISSING.into()))
}

/// Open `path` with grapheme-ui's `desktop::dev_file`, which watches the file
/// and reloads the window in place (RFC-0007 A.9). A broken save keeps the
/// last good UI and shows file, line, column and message over it.
#[cfg(feature = "ui")]
fn launch_dev(path: &str, glyph: Option<&str>) -> Result<(), CompilerError> {
    use grapheme_ui::desktop::{RunFileError, dev_file};
    dev_file(Path::new(path), glyph).map_err(|err| match err {
        RunFileError::Load(load) => CompilerError::RuntimeError(load.to_string()),
        RunFileError::Surface(surface) => {
            CompilerError::RuntimeError(format!("failed to open UI glyph: {surface}"))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_requires_a_file() {
        let err = parse_dev_args(&[]).unwrap_err().to_string();
        assert!(err.contains("file"), "{err}");
    }

    #[test]
    fn dev_accepts_glyph_before_or_after_the_file() {
        let (file, glyph) =
            parse_dev_args(&["--glyph".into(), "CounterApp".into(), "app.gr".into()]).unwrap();
        assert_eq!(file, "app.gr");
        assert_eq!(glyph.as_deref(), Some("CounterApp"));

        let (file, glyph) =
            parse_dev_args(&["app.gr".into(), "--glyph".into(), "Feed".into()]).unwrap();
        assert_eq!(file, "app.gr");
        assert_eq!(glyph.as_deref(), Some("Feed"));
    }

    #[cfg(not(feature = "ui"))]
    #[test]
    fn dev_without_ui_names_the_reinstall() {
        let err = launch_dev("app.gr", None).unwrap_err().to_string();
        assert!(
            err.contains("--features ui") && err.contains("cargo install grapheme-cli"),
            "{err}"
        );
    }

    #[cfg(feature = "ui")]
    #[test]
    fn dev_forwards_glyph_name_to_dev_file_without_opening_a_window() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("grapheme-dev-glyph-{unique}"));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("app.gr");
        std::fs::write(
            &path,
            r#"
state S { n: Int = 0 }
mutation Inc on S { apply { n: $n + 1 } }
view Only on S { Column { Spacer() Text("{$n}") } }
glyph OnlyApp on S { Only }
"#,
        )
        .expect("write ui file");
        let err = launch_dev(path.to_str().expect("utf8 path"), Some("Nope"))
            .unwrap_err()
            .to_string();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            err.contains("Nope"),
            "dev_file should report the glyph it was given: {err}"
        );
    }
}
