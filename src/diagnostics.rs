use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::MakeWriter;

use crate::paths::AppPaths;

const LOG_LIMIT: u64 = 1024 * 1024;

#[derive(Debug, Clone)]
pub struct DiagnosticEvent {
    category: &'static str,
    detail: String,
}

impl DiagnosticEvent {
    pub fn cdp_error(detail: impl Into<String>) -> Self {
        Self {
            category: "cdp_error",
            detail: detail.into(),
        }
    }

    pub fn safe_message(&self) -> String {
        let _ = &self.detail;
        format!("{}: [REDACTED]", self.category)
    }
}

pub struct DiagnosticsGuard {
    _file: Arc<Mutex<File>>,
}

pub fn init_local_logging(paths: &AppPaths) -> anyhow::Result<DiagnosticsGuard> {
    fs::create_dir_all(paths.logs_dir())?;
    let path = paths.logs_dir().join("codex-skin-lite.log");
    rotate_if_needed(&path)?;
    let file = Arc::new(Mutex::new(
        OpenOptions::new().create(true).append(true).open(path)?,
    ));
    let writer = SharedLogWriter(file.clone());
    let _ = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_target(false)
        .with_writer(writer)
        .try_init();
    Ok(DiagnosticsGuard { _file: file })
}

fn rotate_if_needed(path: &std::path::Path) -> anyhow::Result<()> {
    if fs::metadata(path).is_ok_and(|metadata| metadata.len() >= LOG_LIMIT) {
        let third = path.with_extension("log.3");
        let second = path.with_extension("log.2");
        let first = path.with_extension("log.1");
        let _ = fs::remove_file(&third);
        if second.exists() {
            fs::rename(&second, &third)?;
        }
        if first.exists() {
            fs::rename(&first, &second)?;
        }
        fs::rename(path, &first)?;
    }
    Ok(())
}

#[derive(Clone)]
struct SharedLogWriter(Arc<Mutex<File>>);

impl<'a> MakeWriter<'a> for SharedLogWriter {
    type Writer = SharedLogWriteGuard;

    fn make_writer(&'a self) -> Self::Writer {
        SharedLogWriteGuard(self.0.clone())
    }
}

struct SharedLogWriteGuard(Arc<Mutex<File>>);

impl Write for SharedLogWriteGuard {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("log mutex poisoned"))?
            .write(buffer)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("log mutex poisoned"))?
            .flush()
    }
}
