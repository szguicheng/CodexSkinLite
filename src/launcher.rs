use std::path::Path;
use std::process::{Command, Stdio};

use objc2_app_kit::NSWorkspace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchDecision {
    Launch,
    Attach,
    RestartConfirmationRequired,
}

pub fn decide_launch(process_running: bool, cdp_available: bool) -> LaunchDecision {
    match (process_running, cdp_available) {
        (_, true) => LaunchDecision::Attach,
        (true, false) => LaunchDecision::RestartConfirmationRequired,
        (false, false) => LaunchDecision::Launch,
    }
}

pub fn build_open_command(app: &Path, debug_port: u16) -> Vec<String> {
    vec![
        "open".into(),
        "-W".into(),
        "-a".into(),
        app.to_string_lossy().into_owned(),
        "--args".into(),
        "--remote-debugging-address=127.0.0.1".into(),
        format!("--remote-debugging-port={debug_port}"),
    ]
}

pub trait ProcessInspector: Send + Sync {
    fn is_running(&self, app: &Path) -> anyhow::Result<bool>;
    fn terminate(&self, app: &Path) -> anyhow::Result<bool>;
}

pub trait CommandRunner: Send + Sync {
    fn spawn(&self, command: &[String]) -> anyhow::Result<()>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WorkspaceProcessInspector;

impl ProcessInspector for WorkspaceProcessInspector {
    fn is_running(&self, app: &Path) -> anyhow::Result<bool> {
        Ok(find_running_application(app).is_some())
    }

    fn terminate(&self, app: &Path) -> anyhow::Result<bool> {
        Ok(find_running_application(app).is_some_and(|application| application.terminate()))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct OpenCommandRunner;

impl CommandRunner for OpenCommandRunner {
    fn spawn(&self, command: &[String]) -> anyhow::Result<()> {
        let (program, arguments) = command
            .split_first()
            .ok_or_else(|| anyhow::anyhow!("empty launch command"))?;
        let mut child = Command::new(program)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        Ok(())
    }
}

pub struct MacCodexLauncher<I = WorkspaceProcessInspector, R = OpenCommandRunner> {
    inspector: I,
    runner: R,
}

impl Default for MacCodexLauncher {
    fn default() -> Self {
        Self {
            inspector: WorkspaceProcessInspector,
            runner: OpenCommandRunner,
        }
    }
}

impl<I: ProcessInspector, R: CommandRunner> MacCodexLauncher<I, R> {
    pub fn new(inspector: I, runner: R) -> Self {
        Self { inspector, runner }
    }

    pub fn inspect(&self, app: &Path, cdp_available: bool) -> anyhow::Result<LaunchDecision> {
        Ok(decide_launch(
            self.inspector.is_running(app)?,
            cdp_available,
        ))
    }

    pub fn launch(&self, app: &Path, debug_port: u16) -> anyhow::Result<()> {
        validate_codex_bundle(app)?;
        self.runner.spawn(&build_open_command(app, debug_port))
    }

    pub fn terminate_after_confirmation(&self, app: &Path) -> anyhow::Result<()> {
        if !self.inspector.terminate(app)? {
            anyhow::bail!("Codex did not accept the termination request");
        }
        Ok(())
    }
}

pub fn validate_codex_bundle(app: &Path) -> anyhow::Result<()> {
    if app.extension().and_then(|value| value.to_str()) != Some("app") || !app.is_dir() {
        anyhow::bail!("Codex path is not an application bundle");
    }
    let executable = app.join("Contents/MacOS/Codex");
    if !executable.is_file() {
        anyhow::bail!("Codex bundle executable is missing");
    }
    Ok(())
}

fn find_running_application(
    app: &Path,
) -> Option<objc2::rc::Retained<objc2_app_kit::NSRunningApplication>> {
    let workspace = NSWorkspace::sharedWorkspace();
    workspace.runningApplications().iter().find(|application| {
        application
            .bundleURL()
            .and_then(|url| url.path())
            .is_some_and(|path| Path::new(&path.to_string()) == app)
    })
}
