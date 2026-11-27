//! `grapheme dev`: watch a `.gr` file and reload its window in place.
//!
//! File watching is behind the `ui` feature. Without it, the command explains
//! how to reinstall. The headless compiler stays on Rust 1.94.

#[cfg(feature = "ui")]
use std::fs;
#[cfg(feature = "ui")]
use std::path::{Path, PathBuf};
#[cfg(feature = "ui")]
use std::thread;
#[cfg(feature = "ui")]
use std::time::{Duration, Instant};

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

#[cfg(feature = "ui")]
fn launch_dev(path: &str, glyph: Option<&str>) -> Result<(), CompilerError> {
    use std::sync::Mutex;
    use std::sync::mpsc;

    use grapheme_sdk::GraphemeEngine;
    use grapheme_ui::desktop::run;
    use grapheme_ui::{GraphemeSurface, install};
    use rndr_core::{App, Layout, LogicalSize, Schedule, Size, WindowConfig, frame_wake};

    let loaded = grapheme_ui::load_path(Path::new(path), glyph)
        .map_err(|err| CompilerError::RuntimeError(err.to_string()))?;
    let mut app = App::new();
    install(&mut app);
    app.world_mut().insert_resource(WindowConfig {
        title: format!("{} — dev", loaded.glyph),
        size: LogicalSize::new(520.0, 640.0),
        min_size: Some(LogicalSize::new(320.0, 240.0)),
        ..WindowConfig::default()
    });
    let parent = app.world_mut().spawn((Layout {
        padding: 24.0,
        gap: 8.0,
        width: Size::Fill,
        height: Size::Fill,
        ..Layout::column()
    },));
    let surface =
        GraphemeSurface::mount(app.world_mut(), parent, &loaded.source, Some(&loaded.glyph))
            .map_err(|err| CompilerError::RuntimeError(err.to_string()))?;
    surface
        .drive(&mut app)
        .map_err(|err| CompilerError::RuntimeError(err.to_string()))?;

    let (tx, rx) = mpsc::channel();
    let wake = frame_wake(app.world());
    let watch_path = PathBuf::from(path);
    thread::Builder::new()
        .name("grapheme-dev-watch".into())
        .spawn(move || {
            let Ok(mut watch) = FileWatch::open(&watch_path) else {
                return;
            };
            loop {
                if let Some(text) = watch.wait_change(Duration::from_secs(60)) {
                    if tx.send(text).is_err() {
                        break;
                    }
                    wake.wake();
                }
            }
        })
        .map_err(|err| CompilerError::RuntimeError(format!("start file watcher: {err}")))?;

    app.world_mut().insert_resource(DevHost {
        inner: Mutex::new(DevInner {
            rx,
            path: PathBuf::from(path),
            overlay: None,
            engine: GraphemeEngine::builder().build(),
        }),
    });
    app.add_system(Schedule::Update, dev_frame);
    run(app);
    Ok(())
}

#[cfg(feature = "ui")]
struct DevHost {
    inner: std::sync::Mutex<DevInner>,
}

#[cfg(feature = "ui")]
struct DevInner {
    rx: std::sync::mpsc::Receiver<String>,
    path: PathBuf,
    overlay: Option<rndr_core::Entity>,
    engine: grapheme_sdk::GraphemeEngine,
}

#[cfg(feature = "ui")]
fn dev_frame(world: &mut rndr_core::World) {
    use grapheme_ui::with_driven_surface;

    let pending = {
        let Some(host) = world.resource::<DevHost>() else {
            return;
        };
        let Ok(inner) = host.inner.lock() else {
            return;
        };
        let mut latest = None;
        while let Ok(next) = inner.rx.try_recv() {
            latest = Some(next);
        }
        latest
    };
    let Some(source) = pending else {
        return;
    };
    let _ = with_driven_surface(world, |surface, world| {
        apply_save(surface, world, &source);
    });
}

#[cfg(feature = "ui")]
fn apply_save(
    surface: &mut grapheme_ui::GraphemeSurface,
    world: &mut rndr_core::World,
    source: &str,
) {
    let (path, engine_result) = {
        let Some(host) = world.resource::<DevHost>() else {
            return;
        };
        let Ok(inner) = host.inner.lock() else {
            return;
        };
        let result = inner.engine.compile_artifact(source);
        (inner.path.clone(), result)
    };
    match engine_result {
        Ok(artifact) => {
            clear_overlay(world);
            if let Err(err) = surface.session().reload(artifact) {
                show_overlay(world, surface.root(), &format_saved(&path, &err));
                return;
            }
            if let Err(err) = surface.sync(world) {
                show_overlay(world, surface.root(), &format!("{}: {err}", path.display()));
            }
        }
        Err(err) => show_overlay(world, surface.root(), &format_saved(&path, &err)),
    }
}

#[cfg(feature = "ui")]
fn format_saved(path: &Path, err: &grapheme_sdk::GraphemeSdkError) -> String {
    match err.diagnostics().into_iter().find_map(|diag| diag.span) {
        Some(span) => format!("{}:{}:{}: {err}", path.display(), span.line, span.column),
        None => format!("{}: {err}", path.display()),
    }
}

#[cfg(feature = "ui")]
fn clear_overlay(world: &mut rndr_core::World) {
    use rndr_core::despawn;

    let entity = {
        let Some(host) = world.resource::<DevHost>() else {
            return;
        };
        let Ok(mut inner) = host.inner.lock() else {
            return;
        };
        inner.overlay.take()
    };
    if let Some(entity) = entity
        && world.is_alive(entity)
    {
        despawn(world, entity);
    }
}

#[cfg(feature = "ui")]
fn show_overlay(world: &mut rndr_core::World, parent: rndr_core::Entity, text: &str) {
    use rndr_core::{Color, Label, Layout, Semantics, Size, set_parent};

    clear_overlay(world);
    let entity = world.spawn((
        Layout {
            width: Size::Fill,
            height: Size::Hug,
            padding: 12.0,
            ..Layout::column()
        },
        Color::rgb(0.45, 0.12, 0.14),
        Label::new(text),
        Semantics {
            name: Some(text.to_string()),
            description: String::new(),
            readonly: true,
        },
    ));
    set_parent(world, entity, parent);
    if let Some(host) = world.resource::<DevHost>()
        && let Ok(mut inner) = host.inner.lock()
    {
        inner.overlay = Some(entity);
    }
}

/// Directory watch that reports new file contents after a save.
#[cfg(feature = "ui")]
struct FileWatch {
    _watcher: notify::RecommendedWatcher,
    rx: std::sync::mpsc::Receiver<notify::Result<notify::Event>>,
    path: PathBuf,
    last: String,
}

#[cfg(feature = "ui")]
impl FileWatch {
    fn open(path: &Path) -> std::io::Result<Self> {
        use notify::{RecursiveMode, Watcher};

        let last = fs::read_to_string(path).unwrap_or_default();
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = tx.send(event);
        })
        .map_err(std::io::Error::other)?;
        let dir = path
            .parent()
            .filter(|dir| !dir.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        watcher
            .watch(dir, RecursiveMode::NonRecursive)
            .map_err(std::io::Error::other)?;
        Ok(Self {
            _watcher: watcher,
            rx,
            path: path.to_path_buf(),
            last,
        })
    }

    /// Block until `path` contents differ from the last read, or `timeout` elapses.
    fn wait_change(&mut self, timeout: Duration) -> Option<String> {
        use std::sync::mpsc::RecvTimeoutError;

        let deadline = Instant::now() + timeout;
        loop {
            let now = Instant::now();
            if now >= deadline {
                return None;
            }
            match self.rx.recv_timeout(deadline - now) {
                Ok(_) => {
                    thread::sleep(Duration::from_millis(5));
                    while self.rx.try_recv().is_ok() {}
                    let Ok(text) = fs::read_to_string(&self.path) else {
                        continue;
                    };
                    if text != self.last {
                        self.last = text.clone();
                        return Some(text);
                    }
                }
                Err(RecvTimeoutError::Timeout) => return None,
                Err(RecvTimeoutError::Disconnected) => return None,
            }
        }
    }
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
    fn save_is_visible_within_300ms() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("grapheme-dev-watch-{unique}"));
        fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("app.gr");
        fs::write(&path, "state S { n: Int = 1 }\n").expect("seed");
        let mut watch = FileWatch::open(&path).expect("watch");
        let _ = watch.wait_change(Duration::from_millis(40));
        let started = Instant::now();
        fs::write(&path, "state S { n: Int = 2 }\n").expect("save");
        let got = watch.wait_change(Duration::from_millis(300));
        let elapsed = started.elapsed();
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(got.as_deref(), Some("state S { n: Int = 2 }\n"));
        assert!(
            elapsed < Duration::from_millis(300),
            "reload notice took {elapsed:?}"
        );
    }
}
