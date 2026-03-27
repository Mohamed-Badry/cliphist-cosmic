use crate::models::ClipItem;
use crate::utils::stderr_message;
use futures::future::join_all;
use image::{DynamicImage, GenericImageView};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImagePreview {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub async fn decode_page_images(
    entries: Vec<(usize, String)>,
    preview_width: f32,
    preview_height: f32,
) -> Vec<(usize, String, Result<ImagePreview, String>)> {
    let preview_width = preview_dimension(preview_width);
    let preview_height = preview_dimension(preview_height);
    let futures = entries.into_iter().map(move |(index, line)| async move {
        let result = decode_image_preview(&line, preview_width, preview_height).await;
        (index, line, result)
    });
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

async fn decode_image_preview(
    line: &str,
    preview_width: u32,
    preview_height: u32,
) -> Result<ImagePreview, String> {
    let bytes = decode_entry(line).await?;

    tokio::task::spawn_blocking(move || build_image_preview(bytes, preview_width, preview_height))
        .await
        .map_err(|err| format!("Image preview task failed: {err}"))?
}

fn build_image_preview(
    bytes: Vec<u8>,
    preview_width: u32,
    preview_height: u32,
) -> Result<ImagePreview, String> {
    let preview = image::load_from_memory(&bytes)
        .map_err(|err| format!("Failed to decode image preview: {err}"))?;
    let preview = thumbnail_to_fit(preview, preview_width.max(1), preview_height.max(1));
    let preview = preview.to_rgba8();
    let (width, height) = preview.dimensions();

    Ok(ImagePreview {
        width,
        height,
        pixels: preview.into_raw(),
    })
}

fn thumbnail_to_fit(image: DynamicImage, preview_width: u32, preview_height: u32) -> DynamicImage {
    let (width, height) = image.dimensions();

    if width <= preview_width && height <= preview_height {
        return image;
    }

    image.thumbnail(preview_width, preview_height)
}

fn preview_dimension(value: f32) -> u32 {
    value.max(1.0).round().min(u32::MAX as f32) as u32
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

pub async fn wipe_history() -> Result<(), String> {
    let output = Command::new("cliphist")
        .arg("wipe")
        .output()
        .await
        .map_err(|err| format!("Failed to run cliphist wipe: {err}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(stderr_message(
            "cliphist wipe failed",
            String::from_utf8_lossy(&output.stderr).trim(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
    use std::io::Cursor;

    #[test]
    fn build_image_preview_downscales_large_images() {
        let bytes = png_bytes(400, 200);
        let preview = build_image_preview(bytes, 100, 50).expect("preview should decode");

        assert_eq!(preview.width, 100);
        assert_eq!(preview.height, 50);
        assert_eq!(preview.pixels.len(), (100 * 50 * 4) as usize);
    }

    #[test]
    fn build_image_preview_does_not_upscale_small_images() {
        let bytes = png_bytes(40, 20);
        let preview = build_image_preview(bytes, 100, 50).expect("preview should decode");

        assert_eq!(preview.width, 40);
        assert_eq!(preview.height, 20);
        assert_eq!(preview.pixels.len(), (40 * 20 * 4) as usize);
    }

    fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        let image = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(
            width,
            height,
            Rgba([0x66, 0x99, 0xcc, 0xff]),
        ));
        let mut cursor = Cursor::new(Vec::new());
        image
            .write_to(&mut cursor, ImageFormat::Png)
            .expect("png encoding should succeed");
        cursor.into_inner()
    }
}
