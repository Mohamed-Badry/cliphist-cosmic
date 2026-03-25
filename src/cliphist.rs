use crate::models::ClipItem;
use crate::utils::stderr_message;
use std::io::Write;
use std::process::{Command, Stdio};

pub async fn decode_page_images(
    entries: Vec<(usize, String)>,
) -> Vec<(usize, Result<Vec<u8>, String>)> {
    entries
        .into_iter()
        .map(|(index, line)| (index, decode_entry(&line)))
        .collect()
}

pub fn load_history() -> Result<Vec<ClipItem>, String> {
    let output = Command::new("cliphist")
        .arg("list")
        .output()
        .map_err(|err| format!("Failed to run cliphist list: {err}"))?;

    if !output.status.success() {
        return Err(stderr_message(
            "cliphist list failed",
            String::from_utf8_lossy(&output.stderr).trim(),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(ClipItem::parse)
        .collect())
}

pub fn decode_entry(line: &str) -> Result<Vec<u8>, String> {
    let mut child = Command::new("cliphist")
        .arg("decode")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("Failed to run cliphist decode: {err}"))?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "cliphist decode did not expose a stdin pipe.".to_string())?;

        stdin
            .write_all(line.as_bytes())
            .and_then(|_| stdin.write_all(b"\n"))
            .map_err(|err| format!("Failed to send cliphist decode input: {err}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|err| format!("Failed to wait for cliphist decode: {err}"))?;

    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(stderr_message(
            "cliphist decode failed",
            String::from_utf8_lossy(&output.stderr).trim(),
        ))
    }
}

pub fn copy_entry(item: &ClipItem) -> Result<(), String> {
    let payload = decode_entry(&item.line)?;
    let mut command = Command::new("wl-copy");
    if let Some(mime) = item.kind.mime_type() {
        command.arg("--type").arg(mime);
    }

    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| format!("Failed to run wl-copy: {err}"))?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "wl-copy did not expose a stdin pipe.".to_string())?;
        stdin
            .write_all(&payload)
            .map_err(|err| format!("Failed to send clipboard payload to wl-copy: {err}"))?;
    }

    let status = child
        .wait()
        .map_err(|err| format!("Failed to wait for wl-copy: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err("wl-copy failed".to_string())
    }
}

pub fn delete_entry(line: &str) -> Result<(), String> {
    let mut child = Command::new("cliphist")
        .arg("delete")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("Failed to run cliphist delete: {err}"))?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "cliphist delete did not expose a stdin pipe.".to_string())?;
        stdin
            .write_all(line.as_bytes())
            .and_then(|_| stdin.write_all(b"\n"))
            .map_err(|err| format!("Failed to send cliphist delete input: {err}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|err| format!("Failed to wait for cliphist delete: {err}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(stderr_message(
            "cliphist delete failed",
            String::from_utf8_lossy(&output.stderr).trim(),
        ))
    }
}
