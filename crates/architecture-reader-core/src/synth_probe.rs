use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::io::Write;

use crate::synth::{parse_tree, SynthTree};

pub fn default_probe_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/synth-ast-probe.mjs")
}

pub fn probe_enabled_from_env() -> bool {
    std::env::var("ARCHITECTURE_READER_USE_SYNTH")
        .map(|value| value == "1")
        .unwrap_or(false)
}

pub fn probe_synth_tree(path: &Path, script: &Path) -> Result<SynthTree, String> {
    if !script.is_file() {
        return Err(format!("Synth probe script not found: {}", script.display()));
    }

    let payload = serde_json::json!({ "path": path });
    let mut child = Command::new("bun")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("Failed to launch Synth probe: {err}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(payload.to_string().as_bytes())
            .map_err(|err| format!("Failed to write Synth probe request: {err}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|err| format!("Synth probe failed: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Synth probe exited with status {:?}: {stderr}",
            output.status.code()
        ));
    }

    let stdout = String::from_utf8(output.stdout).map_err(|err| format!("Synth probe stdout: {err}"))?;
    parse_tree(&stdout)
}