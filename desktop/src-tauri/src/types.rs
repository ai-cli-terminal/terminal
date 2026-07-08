use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};

use portable_pty::{Child, MasterPty};
use serde::Serialize;

pub(crate) type SharedSession = Arc<Mutex<TerminalSession>>;
pub(crate) type SessionMap = Arc<Mutex<HashMap<String, SharedSession>>>;

#[derive(Clone, Default)]
pub(crate) struct TerminalState {
    pub(crate) sessions: SessionMap,
}

pub(crate) struct TerminalSession {
    pub(crate) master: Box<dyn MasterPty + Send>,
    pub(crate) writer: Box<dyn Write + Send>,
    pub(crate) child: Box<dyn Child + Send + Sync>,
}

#[derive(Clone, Serialize)]
pub(crate) struct TerminalData {
    pub(crate) id: String,
    pub(crate) data: String,
}

#[derive(Clone, Serialize)]
pub(crate) struct TerminalExit {
    pub(crate) id: String,
    pub(crate) status: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FrontendSmokeConfig {
    pub(crate) delay_milliseconds: u32,
    pub(crate) selection_text: String,
    pub(crate) paste_text: String,
    pub(crate) paste_expected_output: String,
    pub(crate) scrollback_lines: u32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeInventory {
    pub(crate) checked_at_epoch_seconds: u64,
    pub(crate) probes: Vec<RuntimeProbe>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeProbe {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) status: String,
    pub(crate) detail: String,
    pub(crate) version: Option<String>,
    pub(crate) path: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DockerAppProbe {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) image: String,
    pub(crate) status: String,
    pub(crate) detail: String,
    pub(crate) shell: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct DockerAppDefinition {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) image: String,
    pub(crate) shell: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AptPackageProbe {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) package_name: String,
    pub(crate) status: String,
    pub(crate) detail: String,
    pub(crate) version: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceProbe {
    pub(crate) status: String,
    pub(crate) detail: String,
    pub(crate) host_path: Option<String>,
    pub(crate) ubuntu_path: Option<String>,
    pub(crate) docker_target: Option<String>,
}

#[derive(Clone)]
pub(crate) struct AptPackageDefinition {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) package_name: &'static str,
}

pub(crate) struct ProbeOutput {
    pub(crate) success: bool,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}
