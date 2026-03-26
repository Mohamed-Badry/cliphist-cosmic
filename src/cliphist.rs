use crate::models::ClipItem;
use crate::utils::stderr_message;
use futures::future::join_all;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub async fn decode_page_images(
    entries: Vec<(usize, String)>,
) -> Vec<(usize, Result<Vec<u8>, String>)> {
    let futures = entries
        .into_iter()
        .map(|(index, line)| async move { (index, decode_entry(&line).await) });
    join_all(futures).await
}

pub fn load_history() -> Result<Vec<ClipItem>, String> {
    let output = std::process::Command::new("cliphist")
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

pub async fn decode_entry(line: &str) -> Result<Vec<u8>, String> {
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
            .await
            .map_err(|err| format!("Failed to send cliphist decode input: {err}"))?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|err| format!("Failed to send cliphist decode input: {err}"))?;
    }

    let output = child
        .wait_with_output()
        .await
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

pub async fn copy_entry(item: &ClipItem) -> Result<(), String> {
    let payload = decode_entry(&item.line).await?;
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
            .await
            .map_err(|err| format!("Failed to send clipboard payload to wl-copy: {err}"))?;
    }

    let status = child
        .wait()
        .await
        .map_err(|err| format!("Failed to wait for wl-copy: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err("wl-copy failed".to_string())
    }
}

pub async fn delete_entry(line: &str) -> Result<(), String> {
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
            .await
            .map_err(|err| format!("Failed to send cliphist delete input: {err}"))?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|err| format!("Failed to send cliphist delete input: {err}"))?;
    }

    let output = child
        .wait_with_output()
        .await
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
