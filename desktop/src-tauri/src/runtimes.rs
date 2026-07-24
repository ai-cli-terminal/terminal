use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use portable_pty::CommandBuilder;
use tauri::AppHandle;

use crate::terminal_cmds::resolve_ash_program;
use crate::types::{
    AptPackageDefinition, AptPackageProbe, DockerAppDefinition, DockerAppProbe, ProbeOutput,
    RuntimeInventory, RuntimeProbe, WorkspaceProbe,
};

const DEFAULT_UBUNTU_DISTRO: &str = "Ubuntu";
const DEFAULT_DOCKER_IMAGE: &str = "ubuntu:24.04";
const DOCKER_WORKSPACE_TARGET: &str = "/workspace";
const MANAGED_NPM_PREFIX: &str = "$HOME/.local/share/ai-terminal/npm-global";
/// PowerShell 7 전용 호스트(폴백 없음, DESIGN D2). Windows PowerShell(`powershell.exe`)는
/// 대상 아님 — pwsh 미설치 시 페인 미표시(probe가 설치 힌트 표시).
const POWERSHELL_PROGRAM: &str = "pwsh.exe";

#[tauri::command]
pub(crate) fn runtime_inventory(app: AppHandle, workspace_dir: Option<String>) -> RuntimeInventory {
    RuntimeInventory {
        checked_at_epoch_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default(),
        probes: vec![
            probe_ash(&app),
            probe_wsl_ubuntu(),
            probe_powershell(),
            probe_docker(workspace_dir.as_deref()),
            probe_managed_ai_cli("codex", "Codex"),
            probe_managed_ai_cli("claude", "Claude"),
            probe_managed_ai_cli("gemini", "Gemini"),
        ],
    }
}

#[tauri::command]
pub(crate) fn workspace_probe(workspace_dir: Option<String>) -> WorkspaceProbe {
    let Some(workspace_dir) = workspace_dir
        .as_deref()
        .map(str::trim)
        .filter(|workspace_dir| !workspace_dir.is_empty())
    else {
        return WorkspaceProbe {
            status: "ready".to_string(),
            detail: "Workspace unset. Ubuntu starts in its default login directory; Docker uses the app working directory for /workspace.".to_string(),
            host_path: None,
            ubuntu_path: None,
            docker_target: Some(DOCKER_WORKSPACE_TARGET.to_string()),
        };
    };

    match explicit_workspace_host_source(workspace_dir) {
        Ok(source) => {
            let source_text = source.display().to_string();
            let ubuntu_path = ubuntu_workspace_dir(Some(&source_text)).ok().flatten();
            WorkspaceProbe {
                status: "ready".to_string(),
                detail: format!(
                    "Workspace ready: {}. Docker mounts it at {DOCKER_WORKSPACE_TARGET}.",
                    source.display()
                ),
                host_path: Some(source_text),
                ubuntu_path,
                docker_target: Some(DOCKER_WORKSPACE_TARGET.to_string()),
            }
        }
        Err(error) => WorkspaceProbe {
            status: "unavailable".to_string(),
            detail: error,
            host_path: None,
            ubuntu_path: None,
            docker_target: None,
        },
    }
}

#[tauri::command]
pub(crate) fn wsl_ubuntu_install() -> Result<String, String> {
    let distro = preferred_ubuntu_distro();
    let mut command = Command::new("wsl.exe");
    command.args(["--install", "-d", &distro]);
    configure_probe_command(&mut command);
    let child = command.spawn().map_err(display_error)?;
    Ok(format!(
        "Started WSL Ubuntu install for {distro} (pid {}). Refresh runtimes after it completes.",
        child.id()
    ))
}

#[tauri::command]
pub(crate) fn apt_package_catalog() -> Vec<AptPackageProbe> {
    let distro = resolve_ubuntu_distro();
    apt_package_definitions()
        .into_iter()
        .map(|definition| match &distro {
            Ok(distro) => {
                let version = apt_package_version(definition.package_name);
                AptPackageProbe {
                    id: definition.id.to_string(),
                    label: definition.label.to_string(),
                    package_name: definition.package_name.to_string(),
                    status: if version.is_some() {
                        "ready"
                    } else {
                        "missing"
                    }
                    .to_string(),
                    detail: if version.is_some() {
                        format!(
                            "{} is installed in managed Ubuntu distro {distro}.",
                            definition.label
                        )
                    } else {
                        format!(
                            "{} is not installed in managed Ubuntu distro {distro}.",
                            definition.label
                        )
                    },
                    version,
                }
            }
            Err(error) => AptPackageProbe {
                id: definition.id.to_string(),
                label: definition.label.to_string(),
                package_name: definition.package_name.to_string(),
                status: "unavailable".to_string(),
                detail: format!("Managed Ubuntu is not ready: {error}"),
                version: None,
            },
        })
        .collect()
}

#[tauri::command]
pub(crate) fn apt_update() -> Result<String, String> {
    let output = run_wsl_bash_probe("sudo -n apt-get update")?;
    if output.success {
        Ok("Updated Ubuntu apt package index.".to_string())
    } else {
        Err(first_non_empty(&output.stderr, &output.stdout)
            .unwrap_or_else(|| "apt-get update failed".to_string()))
    }
}

#[tauri::command]
pub(crate) fn apt_package_install(package_id: String) -> Result<String, String> {
    let definition = apt_package_definition(&package_id)?;
    let script = format!("sudo -n apt-get install -y {}", definition.package_name);
    let output = run_wsl_bash_probe(&script)?;
    if output.success {
        Ok(format!(
            "Installed Ubuntu apt package: {} ({})",
            definition.label, definition.package_name
        ))
    } else {
        Err(first_non_empty(&output.stderr, &output.stdout)
            .unwrap_or_else(|| format!("apt-get install failed for {}", definition.package_name)))
    }
}

#[tauri::command]
pub(crate) fn docker_desktop_install() -> Result<String, String> {
    let mut command = Command::new("winget");
    command.args([
        "install",
        "--exact",
        "--id",
        "Docker.DockerDesktop",
        "--accept-package-agreements",
        "--accept-source-agreements",
    ]);
    configure_probe_command(&mut command);
    let child = command.spawn().map_err(display_error)?;
    Ok(format!(
        "Started Docker Desktop install through winget (pid {}). Refresh runtimes after it completes.",
        child.id()
    ))
}

#[tauri::command]
pub(crate) fn docker_image_pull() -> Result<String, String> {
    let image = preferred_docker_image();
    let output = run_probe("docker", &["pull", &image])?;
    if output.success {
        Ok(format!("Pulled Docker image: {image}"))
    } else {
        Err(first_non_empty(&output.stderr, &output.stdout)
            .unwrap_or_else(|| format!("docker pull failed for {image}")))
    }
}

#[tauri::command]
pub(crate) fn docker_app_catalog(workspace_dir: Option<String>) -> Vec<DockerAppProbe> {
    let engine_ready = docker_engine_ready();
    let workspace_detail = docker_workspace_detail(workspace_dir.as_deref());
    docker_app_definitions()
        .into_iter()
        .map(|definition| {
            let image_ready = engine_ready && docker_image_exists(&definition.image);
            let status = if !engine_ready {
                "unavailable"
            } else if image_ready {
                "ready"
            } else {
                "missing"
            };
            DockerAppProbe {
                id: definition.id.to_string(),
                label: definition.label.to_string(),
                image: definition.image.clone(),
                status: status.to_string(),
                detail: if !engine_ready {
                    format!(
                        "Docker Engine is not reachable. Start Docker Desktop before pulling {}. {workspace_detail}",
                        definition.image,
                    )
                } else if image_ready {
                    format!(
                        "Docker app image is ready: {}. {workspace_detail}",
                        definition.image
                    )
                } else {
                    format!(
                        "Docker app image is missing: {}. {workspace_detail}",
                        definition.image
                    )
                },
                shell: definition.shell,
            }
        })
        .collect()
}

#[tauri::command]
pub(crate) fn docker_app_pull(app_id: String) -> Result<String, String> {
    let definition = docker_app_definition(&app_id)?;
    let output = run_probe("docker", &["pull", &definition.image])?;
    if output.success {
        Ok(format!(
            "Pulled Docker app image for {}: {}",
            definition.label, definition.image
        ))
    } else {
        Err(first_non_empty(&output.stderr, &output.stdout)
            .unwrap_or_else(|| format!("docker pull failed for {}", definition.image)))
    }
}

#[tauri::command]
pub(crate) fn ai_cli_install() -> Result<String, String> {
    run_managed_ai_cli_script(
        &env::var("AI_TERMINAL_AI_CLI_INSTALL_SCRIPT")
            .unwrap_or_else(|_| managed_ai_cli_npm_script("Installing managed AI CLIs")),
        "Installed AI CLIs in managed Ubuntu.",
    )
}

#[tauri::command]
pub(crate) fn ai_cli_update() -> Result<String, String> {
    run_managed_ai_cli_script(
        &env::var("AI_TERMINAL_AI_CLI_UPDATE_SCRIPT")
            .or_else(|_| env::var("AI_TERMINAL_AI_CLI_INSTALL_SCRIPT"))
            .unwrap_or_else(|_| managed_ai_cli_npm_script("Updating managed AI CLIs")),
        "Updated AI CLIs in managed Ubuntu.",
    )
}

fn preferred_ubuntu_distro() -> String {
    env::var("AI_TERMINAL_UBUNTU_DISTRO")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_UBUNTU_DISTRO.to_string())
}

fn resolve_ubuntu_distro() -> Result<String, String> {
    let preferred = preferred_ubuntu_distro();
    let output = run_probe("wsl.exe", &["--list", "--quiet"])?;
    if !output.success {
        return Err(first_non_empty(&output.stderr, &output.stdout)
            .unwrap_or_else(|| "wsl.exe did not list installed distributions".to_string()));
    }

    let distros = output
        .stdout
        .lines()
        .map(clean_wsl_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    if distros
        .iter()
        .any(|distro| distro.eq_ignore_ascii_case(&preferred))
    {
        return Ok(preferred);
    }

    distros
        .into_iter()
        .find(|distro| distro.to_ascii_lowercase().contains("ubuntu"))
        .ok_or_else(|| {
            format!("No Ubuntu WSL distribution is installed. Install {preferred} first.")
        })
}

pub(crate) fn wsl_ubuntu_command(workspace_dir: Option<&str>) -> Result<CommandBuilder, String> {
    let distro = resolve_ubuntu_distro()?;
    let mut command = CommandBuilder::new("wsl.exe");
    if let Some(workspace_dir) = ubuntu_workspace_dir(workspace_dir)? {
        let quoted = bash_quote(&workspace_dir);
        let script = format!(
            "cd {quoted} 2>/dev/null || printf 'Workspace not found: %s\\n' {quoted}\nexec bash -l"
        );
        command.args(["-d", &distro, "--exec", "bash", "-lc", &script]);
    } else {
        command.args(["-d", &distro, "--exec", "bash", "-l"]);
    }
    Ok(command)
}

fn wsl_bash_command(script: &str) -> Result<CommandBuilder, String> {
    let distro = resolve_ubuntu_distro()?;
    let mut command = CommandBuilder::new("wsl.exe");
    command.args(["-d", &distro, "--exec", "bash", "-lc", script]);
    Ok(command)
}

/// PowerShell 페인의 순수 실행 계획. `CommandBuilder`는 필드를 되읽을 수 없어
/// (winexec.rs 패턴) 이 순수 구조체로 workspace 유무 분기를 단위테스트한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PowershellPlan {
    pub program: String,
    /// workspace가 지정되면 Windows 호스트 cwd. 미지정이면 pwsh 기본 디렉터리.
    pub cwd: Option<String>,
}

/// pwsh 7 전용(폴백 없음). WSL 페인과 달리 Windows 호스트에서 직접 실행하므로
/// `Set-Location` 스크립트 주입 없이 cwd로 workspace를 연다(DESIGN D2).
pub(crate) fn powershell_plan(workspace_dir: Option<&str>) -> PowershellPlan {
    let cwd = workspace_dir
        .map(str::trim)
        .filter(|dir| !dir.is_empty())
        .map(ToOwned::to_owned);
    PowershellPlan {
        program: POWERSHELL_PROGRAM.to_string(),
        cwd,
    }
}

pub(crate) fn powershell_command(workspace_dir: Option<&str>) -> CommandBuilder {
    let plan = powershell_plan(workspace_dir);
    let mut command = CommandBuilder::new(&plan.program);
    if let Some(cwd) = plan.cwd {
        command.cwd(cwd);
    }
    command
}

fn run_wsl_bash_probe(script: &str) -> Result<ProbeOutput, String> {
    let distro = resolve_ubuntu_distro()?;
    run_probe("wsl.exe", &["-d", &distro, "--exec", "bash", "-lc", script])
}

fn explicit_workspace_host_source(workspace_dir: &str) -> Result<PathBuf, String> {
    if cfg!(windows) && workspace_dir.starts_with('/') {
        return Err(
            "Workspace must be a Windows host directory for shared Ubuntu/Docker panes."
                .to_string(),
        );
    }
    if workspace_dir.starts_with(r"\\") {
        return Err("Workspace does not support UNC paths yet.".to_string());
    }

    let source = PathBuf::from(workspace_dir);
    let source = if source.is_absolute() {
        source
    } else {
        env::current_dir().map_err(display_error)?.join(source)
    };
    if !source.is_dir() {
        return Err(format!(
            "Workspace source is not a directory: {}",
            source.display()
        ));
    }
    Ok(source)
}

fn ubuntu_workspace_dir(workspace_dir: Option<&str>) -> Result<Option<String>, String> {
    let Some(workspace_dir) = workspace_dir
        .map(str::trim)
        .filter(|workspace_dir| !workspace_dir.is_empty())
    else {
        return Ok(None);
    };

    if workspace_dir.starts_with('/') {
        return Ok(Some(workspace_dir.to_string()));
    }
    if workspace_dir.starts_with(r"\\") {
        return Err("Ubuntu workspace does not support UNC paths yet.".to_string());
    }
    if let Some(path) = windows_path_to_wsl(workspace_dir) {
        return Ok(Some(path));
    }

    let source = env::current_dir()
        .map_err(display_error)?
        .join(PathBuf::from(workspace_dir));
    let source = source.display().to_string();
    if source.starts_with('/') {
        return Ok(Some(source));
    }
    windows_path_to_wsl(&source)
        .map(Some)
        .ok_or_else(|| format!("Could not map workspace path into Ubuntu: {workspace_dir}"))
}

fn windows_path_to_wsl(path: &str) -> Option<String> {
    let mut chars = path.chars();
    let drive = chars.next()?;
    if !drive.is_ascii_alphabetic() || chars.next()? != ':' {
        return None;
    }

    let rest = chars.as_str();
    let rest = rest.strip_prefix('\\').or_else(|| rest.strip_prefix('/'))?;
    let rest = rest.replace('\\', "/");
    if rest.is_empty() {
        Some(format!("/mnt/{}", drive.to_ascii_lowercase()))
    } else {
        Some(format!("/mnt/{}/{}", drive.to_ascii_lowercase(), rest))
    }
}

fn bash_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'\''"#))
}

pub(crate) fn ai_cli_runtime_command(
    runtime: &str,
    workspace_dir: Option<&str>,
) -> Result<CommandBuilder, String> {
    let label = match runtime {
        "codex" => "Codex",
        "claude" => "Claude",
        "gemini" => "Gemini",
        _ => return Err(format!("unknown AI CLI runtime: {runtime}")),
    };
    let workspace_script = ubuntu_workspace_dir(workspace_dir)?
        .map(|workspace_dir| {
            let quoted = bash_quote(&workspace_dir);
            format!("cd {quoted} 2>/dev/null || printf 'Workspace not found: %s\\n' {quoted}\n")
        })
        .unwrap_or_default();
    let script = format!(
        r#"export PATH="{MANAGED_NPM_PREFIX}/bin:$PATH"
{workspace_script}if ! command -v {runtime} >/dev/null 2>&1; then
  echo "{label} CLI is not installed in managed Ubuntu. Use Install AI CLIs from the ribbon." >&2
  exec bash -l
fi
exec {runtime}
"#
    );
    wsl_bash_command(&script)
}

fn managed_ai_cli_npm_script(action: &str) -> String {
    format!(
        r##"set -e
echo "{action}"
if ! command -v npm >/dev/null 2>&1; then
  echo "npm is missing; installing nodejs/npm through Ubuntu apt"
  sudo -n apt-get update
  sudo -n apt-get install -y nodejs npm
fi
mkdir -p "{MANAGED_NPM_PREFIX}"
npm config set prefix "{MANAGED_NPM_PREFIX}"
export PATH="{MANAGED_NPM_PREFIX}/bin:$PATH"
profile="$HOME/.profile"
start="# >>> ai-terminal ai-cli path >>>"
end="# <<< ai-terminal ai-cli path <<<"
if [ -f "$profile" ] && grep -Fq "$start" "$profile"; then
  :
else
  {{
    printf '\n%s\n' "$start"
    printf 'export PATH="{MANAGED_NPM_PREFIX}/bin:$PATH"\n'
    printf '%s\n' "$end"
  }} >> "$profile"
fi
npm install -g @openai/codex@latest @anthropic-ai/claude-code@latest @google/gemini-cli@latest
codex --version || true
claude --version || true
gemini --version || true
"##
    )
}

fn run_managed_ai_cli_script(script: &str, success_message: &str) -> Result<String, String> {
    let output = run_wsl_bash_probe(script)?;
    if output.success {
        let detail = first_non_empty(&output.stdout, &output.stderr)
            .map(|line| format!(" {line}"))
            .unwrap_or_default();
        Ok(format!("{success_message}{detail}"))
    } else {
        Err(first_non_empty(&output.stderr, &output.stdout)
            .unwrap_or_else(|| "managed AI CLI script failed".to_string()))
    }
}

fn apt_package_definitions() -> Vec<AptPackageDefinition> {
    vec![
        AptPackageDefinition {
            id: "git",
            label: "Git",
            package_name: "git",
        },
        AptPackageDefinition {
            id: "curl",
            label: "curl",
            package_name: "curl",
        },
        AptPackageDefinition {
            id: "build-essential",
            label: "Build Essential",
            package_name: "build-essential",
        },
        AptPackageDefinition {
            id: "python3",
            label: "Python 3",
            package_name: "python3",
        },
        AptPackageDefinition {
            id: "nodejs",
            label: "Node.js",
            package_name: "nodejs",
        },
        AptPackageDefinition {
            id: "npm",
            label: "npm",
            package_name: "npm",
        },
    ]
}

fn apt_package_definition(package_id: &str) -> Result<AptPackageDefinition, String> {
    apt_package_definitions()
        .into_iter()
        .find(|definition| definition.id == package_id)
        .ok_or_else(|| format!("unknown apt package: {package_id}"))
}

fn apt_package_version(package_name: &str) -> Option<String> {
    let script = format!("dpkg-query -W -f='${{Version}}' {package_name}");
    run_wsl_bash_probe(&script)
        .ok()
        .filter(|output| output.success)
        .and_then(|output| first_non_empty(&output.stdout, &output.stderr))
}

fn preferred_docker_image() -> String {
    env::var("AI_TERMINAL_DOCKER_IMAGE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_DOCKER_IMAGE.to_string())
}

fn preferred_docker_shell() -> Vec<String> {
    env::var("AI_TERMINAL_DOCKER_SHELL")
        .ok()
        .map(|value| {
            value
                .split_whitespace()
                .filter(|part| !part.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|parts| !parts.is_empty())
        .unwrap_or_else(|| vec!["bash".to_string(), "-l".to_string()])
}

fn docker_image_exists(image: &str) -> bool {
    run_probe(
        "docker",
        &["image", "inspect", image, "--format", "{{.Id}}"],
    )
    .map(|output| output.success)
    .unwrap_or(false)
}

pub(crate) fn docker_runtime_command(workspace_dir: Option<&str>) -> Result<CommandBuilder, String> {
    let image = preferred_docker_image();
    if !docker_engine_ready() {
        return Err("Docker Engine is not reachable. Start Docker Desktop first.".to_string());
    }
    if !docker_image_exists(&image) {
        return Err(format!(
            "Docker image {image} is not present. Use Pull Image before starting Docker runtime."
        ));
    }

    let shell = preferred_docker_shell();
    let mut command = CommandBuilder::new("docker");
    command.args(["run", "--rm", "-it"]);
    add_docker_workspace_args(&mut command, workspace_dir)?;
    command.arg(&image);
    command.args(shell);
    Ok(command)
}

fn docker_app_definitions() -> Vec<DockerAppDefinition> {
    vec![
        DockerAppDefinition {
            id: "ubuntu-base",
            label: "Ubuntu Base",
            image: preferred_docker_image(),
            shell: vec!["bash".to_string(), "-l".to_string()],
        },
        DockerAppDefinition {
            id: "node-dev",
            label: "Node.js Dev",
            image: env::var("AI_TERMINAL_DOCKER_APP_NODE_IMAGE")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "node:22-bookworm".to_string()),
            shell: vec!["bash".to_string(), "-l".to_string()],
        },
        DockerAppDefinition {
            id: "python-dev",
            label: "Python Dev",
            image: env::var("AI_TERMINAL_DOCKER_APP_PYTHON_IMAGE")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "python:3.12-bookworm".to_string()),
            shell: vec!["bash".to_string(), "-l".to_string()],
        },
        DockerAppDefinition {
            id: "rust-dev",
            label: "Rust Dev",
            image: env::var("AI_TERMINAL_DOCKER_APP_RUST_IMAGE")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "rust:1-bookworm".to_string()),
            shell: vec!["bash".to_string(), "-l".to_string()],
        },
    ]
}

fn docker_app_definition(app_id: &str) -> Result<DockerAppDefinition, String> {
    docker_app_definitions()
        .into_iter()
        .find(|definition| definition.id == app_id)
        .ok_or_else(|| format!("unknown Docker app: {app_id}"))
}

pub(crate) fn docker_app_runtime_command(
    app_id: &str,
    workspace_dir: Option<&str>,
) -> Result<CommandBuilder, String> {
    let definition = docker_app_definition(app_id)?;
    if !docker_engine_ready() {
        return Err("Docker Engine is not reachable. Start Docker Desktop first.".to_string());
    }
    if !docker_image_exists(&definition.image) {
        return Err(format!(
            "Docker app image {} is not present. Use Pull App before starting {}.",
            definition.image, definition.label
        ));
    }

    let mut command = CommandBuilder::new("docker");
    command.args(["run", "--rm", "-it"]);
    add_docker_workspace_args(&mut command, workspace_dir)?;
    command.arg(&definition.image);
    command.args(definition.shell);
    Ok(command)
}

fn add_docker_workspace_args(
    command: &mut CommandBuilder,
    workspace_dir: Option<&str>,
) -> Result<(), String> {
    let Some(source) = docker_workspace_source(workspace_dir)? else {
        return Ok(());
    };

    command.arg("--mount");
    command.arg(format!(
        "type=bind,source={},target={DOCKER_WORKSPACE_TARGET}",
        source.display()
    ));
    command.args(["--workdir", DOCKER_WORKSPACE_TARGET]);
    Ok(())
}

fn docker_workspace_source(workspace_dir: Option<&str>) -> Result<Option<PathBuf>, String> {
    if !docker_workspace_mount_enabled() {
        return Ok(None);
    }

    if let Some(workspace_dir) = workspace_dir
        .map(str::trim)
        .filter(|workspace_dir| !workspace_dir.is_empty())
    {
        return explicit_workspace_host_source(workspace_dir).map(Some);
    }

    let source = workspace_dir
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("AI_TERMINAL_WORKSPACE_DIR").map(PathBuf::from))
        .map(Ok)
        .unwrap_or_else(env::current_dir)
        .map_err(display_error)?;
    let source = if source.is_absolute() {
        source
    } else {
        env::current_dir().map_err(display_error)?.join(source)
    };
    if !source.is_dir() {
        return Err(format!(
            "Docker workspace source is not a directory: {}",
            source.display()
        ));
    }
    Ok(Some(source))
}

fn docker_workspace_mount_enabled() -> bool {
    env::var("AI_TERMINAL_DOCKER_WORKSPACE")
        .ok()
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "no" | "off"
            )
        })
        .unwrap_or(true)
}

fn docker_workspace_detail(workspace_dir: Option<&str>) -> String {
    match docker_workspace_source(workspace_dir) {
        Ok(Some(source)) => format!(
            "Workspace mount: {} -> {DOCKER_WORKSPACE_TARGET}.",
            source.display()
        ),
        Ok(None) => "Workspace mount disabled.".to_string(),
        Err(error) => format!("Workspace mount unavailable: {error}"),
    }
}

fn probe_ash(app: &AppHandle) -> RuntimeProbe {
    match resolve_ash_program(app) {
        Ok(path) => {
            let exists = path.exists();
            RuntimeProbe {
                id: "ash".to_string(),
                label: "ash".to_string(),
                status: if exists { "ready" } else { "unknown" }.to_string(),
                detail: if exists {
                    "Bundled ash sidecar was found.".to_string()
                } else {
                    "ash will be resolved from PATH when the terminal opens.".to_string()
                },
                version: None,
                path: Some(path.display().to_string()),
            }
        }
        Err(error) => RuntimeProbe {
            id: "ash".to_string(),
            label: "ash".to_string(),
            status: "unavailable".to_string(),
            detail: error.to_string(),
            version: None,
            path: None,
        },
    }
}

fn probe_wsl_ubuntu() -> RuntimeProbe {
    let path = find_program_path("wsl.exe");
    let preferred = preferred_ubuntu_distro();
    let status = match run_probe("wsl.exe", &["--status"]) {
        Ok(output) if output.success => output,
        Ok(output) => {
            return RuntimeProbe {
                id: "ubuntu".to_string(),
                label: "Ubuntu".to_string(),
                status: "unavailable".to_string(),
                detail: first_non_empty(&output.stderr, &output.stdout).unwrap_or_else(|| {
                    "wsl.exe is installed but did not report a usable status.".to_string()
                }),
                version: None,
                path,
            };
        }
        Err(error) => {
            return RuntimeProbe {
                id: "ubuntu".to_string(),
                label: "Ubuntu".to_string(),
                status: "unavailable".to_string(),
                detail: error,
                version: None,
                path,
            };
        }
    };

    let distros = run_probe("wsl.exe", &["--list", "--verbose"]).ok();
    let ubuntu_line = distros
        .as_ref()
        .and_then(|output| find_ubuntu_distro_line(&output.stdout));
    let distro_detail = ubuntu_line.as_ref().map(|line| {
        if line
            .to_ascii_lowercase()
            .contains(&preferred.to_ascii_lowercase())
        {
            format!("Managed distro ready: {line}")
        } else {
            format!("Ubuntu distro ready: {line}")
        }
    });
    RuntimeProbe {
        id: "ubuntu".to_string(),
        label: "Ubuntu".to_string(),
        status: if ubuntu_line.is_some() {
            "ready"
        } else {
            "missing"
        }
        .to_string(),
        detail: distro_detail.unwrap_or_else(|| {
            first_non_empty(&status.stdout, &status.stderr)
                .map(|line| {
                    format!(
                        "WSL is available, but no Ubuntu distro was found. Preferred distro: {preferred}. {line}"
                    )
                })
                .unwrap_or_else(|| {
                    format!(
                        "WSL is available, but no Ubuntu distro was found. Preferred distro: {preferred}."
                    )
                })
        }),
        version: extract_wsl_version(&status.stdout),
        path,
    }
}

fn probe_powershell() -> RuntimeProbe {
    let path = find_program_path(POWERSHELL_PROGRAM);
    match run_probe(POWERSHELL_PROGRAM, &["--version"]) {
        Ok(output) if output.success => RuntimeProbe {
            id: "powershell".to_string(),
            label: "PowerShell".to_string(),
            status: "ready".to_string(),
            detail: "PowerShell 7 (pwsh) is available.".to_string(),
            version: first_non_empty(&output.stdout, &output.stderr),
            path,
        },
        // pwsh 미설치(spawn 실패) 또는 비정상 종료 → missing + 설치 힌트(DESIGN D2, WSL "Install Ubuntu" 대칭).
        _ => RuntimeProbe {
            id: "powershell".to_string(),
            label: "PowerShell".to_string(),
            status: "missing".to_string(),
            detail: "PowerShell 7 (pwsh) was not found. Install it with: winget install --id Microsoft.PowerShell"
                .to_string(),
            version: None,
            path,
        },
    }
}

fn probe_docker(workspace_dir: Option<&str>) -> RuntimeProbe {
    let path = find_program_path("docker");
    let image = preferred_docker_image();
    let workspace_detail = docker_workspace_detail(workspace_dir);
    match run_probe("docker", &["--version"]) {
        Ok(output) if output.success => {
            let version = first_non_empty(&output.stdout, &output.stderr);
            let engine = docker_engine_version();
            if engine.is_none() {
                return RuntimeProbe {
                    id: "docker".to_string(),
                    label: "Docker".to_string(),
                    status: "unavailable".to_string(),
                    detail: format!(
                        "Docker CLI is available, but Docker Engine is not reachable. Start Docker Desktop. Managed image: {image}. {workspace_detail}"
                    ),
                    version,
                    path,
                };
            }

            let image_ready = docker_image_exists(&image);
            RuntimeProbe {
                id: "docker".to_string(),
                label: "Docker".to_string(),
                status: if image_ready { "ready" } else { "missing" }.to_string(),
                detail: if image_ready {
                    format!(
                        "Docker Engine is reachable. Managed image is ready: {image}. {workspace_detail}"
                    )
                } else {
                    format!(
                        "Docker Engine is reachable. Managed image is missing: {image}. {workspace_detail}"
                    )
                },
                version: engine.or(version),
                path,
            }
        }
        Ok(output) => RuntimeProbe {
            id: "docker".to_string(),
            label: "Docker".to_string(),
            status: "unavailable".to_string(),
            detail: first_non_empty(&output.stderr, &output.stdout)
                .unwrap_or_else(|| "Docker CLI did not return a version.".to_string()),
            version: None,
            path,
        },
        Err(error) => RuntimeProbe {
            id: "docker".to_string(),
            label: "Docker".to_string(),
            status: "unavailable".to_string(),
            detail: error,
            version: None,
            path,
        },
    }
}

fn docker_engine_ready() -> bool {
    docker_engine_version().is_some()
}

fn docker_engine_version() -> Option<String> {
    run_probe("docker", &["info", "--format", "{{.ServerVersion}}"])
        .ok()
        .filter(|output| output.success)
        .and_then(|output| first_non_empty(&output.stdout, &output.stderr))
}

fn probe_managed_ai_cli(command: &str, label: &str) -> RuntimeProbe {
    let distro = match resolve_ubuntu_distro() {
        Ok(distro) => distro,
        Err(error) => {
            return RuntimeProbe {
                id: command.to_string(),
                label: label.to_string(),
                status: "missing".to_string(),
                detail: format!("Managed Ubuntu is not ready: {error}"),
                version: None,
                path: None,
            };
        }
    };

    let script = format!(
        r#"export PATH="{MANAGED_NPM_PREFIX}/bin:$PATH"
if command -v {command} >/dev/null 2>&1; then
  printf '__AI_TERMINAL_PATH__%s\n' "$(command -v {command})"
  {command} --version
else
  exit 127
fi
"#
    );

    match run_probe("wsl.exe", &["-d", &distro, "--exec", "bash", "-lc", &script]) {
        Ok(output) if output.success => RuntimeProbe {
            id: command.to_string(),
            label: label.to_string(),
            status: "ready".to_string(),
            detail: format!("{label} CLI is installed in managed Ubuntu distro {distro}."),
            version: ai_cli_probe_version(&output.stdout).or_else(|| first_non_empty(&output.stderr, "")),
            path: ai_cli_probe_path(&output.stdout),
        },
        Ok(output) => RuntimeProbe {
            id: command.to_string(),
            label: label.to_string(),
            status: "missing".to_string(),
            detail: first_non_empty(&output.stderr, &output.stdout).unwrap_or_else(|| {
                format!(
                    "{label} CLI is not installed in managed Ubuntu distro {distro}. Use Install AI CLIs."
                )
            }),
            version: None,
            path: None,
        },
        Err(error) => RuntimeProbe {
            id: command.to_string(),
            label: label.to_string(),
            status: "missing".to_string(),
            detail: format!("{label} CLI probe failed in managed Ubuntu distro {distro}: {error}"),
            version: None,
            path: None,
        },
    }
}

fn ai_cli_probe_path(stdout: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        line.strip_prefix("__AI_TERMINAL_PATH__")
            .map(|path| path.trim().to_string())
            .filter(|path| !path.is_empty())
    })
}

fn ai_cli_probe_version(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("__AI_TERMINAL_PATH__"))
        .map(ToOwned::to_owned)
}

fn run_probe(program: &str, args: &[&str]) -> Result<ProbeOutput, String> {
    let mut command = Command::new(program);
    command.args(args);
    configure_probe_command(&mut command);
    let output = command.output().map_err(display_error)?;
    Ok(ProbeOutput {
        success: output.status.success(),
        stdout: decode_process_output(&output.stdout),
        stderr: decode_process_output(&output.stderr),
    })
}

#[cfg(windows)]
fn configure_probe_command(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn configure_probe_command(_command: &mut Command) {}

fn find_program_path(program: &str) -> Option<String> {
    let finder = if cfg!(windows) { "where.exe" } else { "which" };
    run_probe(finder, &[program])
        .ok()
        .filter(|output| output.success)
        .and_then(|output| first_non_empty(&output.stdout, &output.stderr))
}

fn decode_process_output(bytes: &[u8]) -> String {
    if bytes.len() >= 4 {
        let zeros = bytes.iter().filter(|byte| **byte == 0).count();
        if zeros > bytes.len() / 4 {
            let utf16 = bytes
                .chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .collect::<Vec<_>>();
            return String::from_utf16_lossy(&utf16);
        }
    }
    String::from_utf8_lossy(bytes).into_owned()
}

fn first_non_empty(primary: &str, fallback: &str) -> Option<String> {
    primary
        .lines()
        .chain(fallback.lines())
        .map(clean_wsl_line)
        .find(|line| !line.is_empty())
}

fn find_ubuntu_distro_line(output: &str) -> Option<String> {
    output
        .lines()
        .map(clean_wsl_line)
        .map(|line| line.trim_start_matches('*').trim().to_string())
        .find(|line| line.to_ascii_lowercase().contains("ubuntu"))
}

fn clean_wsl_line(line: &str) -> String {
    line.trim().trim_matches('\0').to_string()
}

fn extract_wsl_version(output: &str) -> Option<String> {
    output
        .lines()
        .map(clean_wsl_line)
        .find(|line| line.to_ascii_lowercase().contains("version"))
}

pub(crate) fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powershell_plan_without_workspace_uses_default_cwd() {
        let plan = powershell_plan(None);
        assert_eq!(plan.program, "pwsh.exe");
        assert_eq!(plan.cwd, None);
    }

    #[test]
    fn powershell_plan_blank_workspace_is_treated_as_unset() {
        assert_eq!(powershell_plan(Some("   ")).cwd, None);
        assert_eq!(powershell_plan(Some("")).cwd, None);
    }

    #[test]
    fn powershell_plan_with_workspace_sets_host_cwd() {
        let plan = powershell_plan(Some(r"C:\work\proj"));
        assert_eq!(plan.program, "pwsh.exe");
        assert_eq!(plan.cwd, Some(r"C:\work\proj".to_string()));
    }

    #[test]
    fn powershell_plan_trims_surrounding_whitespace() {
        assert_eq!(
            powershell_plan(Some("  C:\\work  ")).cwd,
            Some(r"C:\work".to_string())
        );
    }
}
