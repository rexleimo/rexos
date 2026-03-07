use anyhow::{bail, Context};

use crate::Toolset;

impl Toolset {
    pub(crate) fn image_analyze(&self, user_path: &str) -> anyhow::Result<String> {
        let path = self.resolve_workspace_path(user_path)?;
        let meta = std::fs::metadata(&path).with_context(|| format!("stat {}", path.display()))?;
        if meta.len() > 10_000_000 {
            bail!("image too large: {} bytes", meta.len());
        }

        let bytes = std::fs::read(&path).with_context(|| format!("read {}", path.display()))?;
        let Some((format, width, height)) = detect_image_format_and_dimensions(&bytes) else {
            bail!("unsupported image format (expected png/jpeg/gif)");
        };

        Ok(serde_json::json!({
            "path": user_path,
            "format": format,
            "width": width,
            "height": height,
            "bytes": bytes.len(),
        })
        .to_string())
    }

    pub(crate) fn media_describe(&self, user_path: &str) -> anyhow::Result<String> {
        let path = self.resolve_workspace_path(user_path)?;
        let meta = std::fs::metadata(&path).with_context(|| format!("stat {}", path.display()))?;
        if meta.len() > 200_000_000 {
            bail!("media too large: {} bytes", meta.len());
        }

        let ext = path
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let kind = match ext.as_str() {
            "wav" | "mp3" | "flac" | "ogg" | "m4a" | "aac" | "opus" => "audio",
            "mp4" | "mov" | "mkv" | "webm" | "avi" => "video",
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" => "image",
            "txt" | "md" | "srt" | "vtt" => "text",
            _ => "unknown",
        };

        Ok(serde_json::json!({
            "path": user_path,
            "bytes": meta.len(),
            "kind": kind,
            "ext": if ext.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(ext) },
        })
        .to_string())
    }

    pub(crate) fn media_transcribe(&self, user_path: &str) -> anyhow::Result<String> {
        let path = self.resolve_workspace_path(user_path)?;
        let meta = std::fs::metadata(&path).with_context(|| format!("stat {}", path.display()))?;
        if meta.len() > 2_000_000 {
            bail!("transcript too large: {} bytes", meta.len());
        }

        let ext = path
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        match ext.as_str() {
            "txt" | "md" | "srt" | "vtt" => {}
            _ => bail!("media_transcribe currently supports text transcripts (.txt/.md/.srt/.vtt)"),
        }

        let raw =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let text = raw.trim_end_matches(&['\r', '\n'][..]).to_string();

        Ok(serde_json::json!({
            "path": user_path,
            "text": text,
        })
        .to_string())
    }

    pub(crate) fn speech_to_text(&self, user_path: &str) -> anyhow::Result<String> {
        let out = self.media_transcribe(user_path)?;
        let v: serde_json::Value =
            serde_json::from_str(&out).context("parse media_transcribe output")?;
        let text = v
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        Ok(serde_json::json!({
            "path": user_path,
            "transcript": text,
            "text": v.get("text").cloned().unwrap_or(serde_json::Value::Null),
            "note": "MVP: speech_to_text currently supports transcript files (.txt/.md/.srt/.vtt).",
        })
        .to_string())
    }

    pub(crate) fn text_to_speech(&self, text: &str, path: Option<&str>) -> anyhow::Result<String> {
        if text.trim().is_empty() {
            bail!("text is empty");
        }

        let rel = path.unwrap_or(".loopforge/audio/tts.wav");
        let out_path = self.resolve_workspace_path_for_write(rel)?;
        if out_path.extension().and_then(|x| x.to_str()).unwrap_or("") != "wav" {
            bail!("text_to_speech currently only supports .wav output paths");
        }

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dirs {}", parent.display()))?;
        }

        let sample_rate: u32 = 16_000;
        let duration_ms: u32 = 300;
        let num_samples = (sample_rate as usize)
            .saturating_mul(duration_ms as usize)
            .saturating_div(1000);
        let frequency_hz: f32 = 440.0;
        let amplitude: f32 = 0.20;

        let data_size = num_samples.saturating_mul(2);
        let riff_size = 36u32.saturating_add(data_size as u32);

        let mut bytes = Vec::with_capacity(44 + data_size);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&riff_size.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&1u16.to_le_bytes()); // channels
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        let byte_rate = sample_rate.saturating_mul(2);
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes()); // block align
        bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(data_size as u32).to_le_bytes());

        for n in 0..num_samples {
            let t = n as f32 / sample_rate as f32;
            let s = (2.0 * std::f32::consts::PI * frequency_hz * t).sin();
            let sample = (s * amplitude * i16::MAX as f32) as i16;
            bytes.extend_from_slice(&sample.to_le_bytes());
        }

        std::fs::write(&out_path, &bytes)
            .with_context(|| format!("write {}", out_path.display()))?;

        Ok(serde_json::json!({
            "path": rel,
            "format": "wav",
            "bytes": bytes.len(),
            "note": "MVP: generates a short WAV tone (placeholder for real TTS).",
        })
        .to_string())
    }

    pub(crate) fn image_generate(&self, prompt: &str, user_path: &str) -> anyhow::Result<String> {
        if prompt.trim().is_empty() {
            bail!("prompt is empty");
        }

        let out_path = self.resolve_workspace_path_for_write(user_path)?;
        if out_path.extension().and_then(|x| x.to_str()).unwrap_or("") != "svg" {
            bail!("only svg output is supported for now (use a .svg path)");
        }

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dirs {}", parent.display()))?;
        }

        let escaped = escape_xml_text(prompt);
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="800" height="450" viewBox="0 0 800 450"><rect width="100%" height="100%" fill="#0b1020"/><text x="40" y="120" fill="#e2e8f0" font-size="48" font-family="Inter, system-ui, -apple-system, Segoe UI, Roboto, Arial">{escaped}</text></svg>"##
        );

        std::fs::write(&out_path, svg).with_context(|| format!("write {}", out_path.display()))?;

        Ok(serde_json::json!({
            "path": user_path,
            "format": "svg",
        })
        .to_string())
    }

    pub(crate) fn canvas_present(&self, html: &str, title: Option<&str>) -> anyhow::Result<String> {
        let title = title
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .unwrap_or("Canvas");

        let sanitized = sanitize_canvas_html(html, 512 * 1024)?;

        let canvas_id = uuid::Uuid::new_v4().to_string();
        let rel = format!("output/canvas_{canvas_id}.html");
        let out_path = self.resolve_workspace_path_for_write(&rel)?;
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create dirs {}", parent.display()))?;
        }

        let safe_title = escape_xml_text(title);
        let full = format!(
            "<!DOCTYPE html>\n<html>\n<head><meta charset=\"utf-8\"><title>{safe_title}</title></head>\n<body>\n{sanitized}\n</body>\n</html>\n"
        );

        std::fs::write(&out_path, &full)
            .with_context(|| format!("write {}", out_path.display()))?;

        Ok(serde_json::json!({
            "canvas_id": canvas_id,
            "title": title,
            "saved_to": rel,
            "size_bytes": full.len(),
        })
        .to_string())
    }
}

fn escape_xml_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

fn contains_event_handler_attr(lower: &str) -> bool {
    let bytes = lower.as_bytes();
    for i in 0..bytes.len().saturating_sub(2) {
        if bytes[i] != b'o' || bytes[i + 1] != b'n' {
            continue;
        }

        if i > 0 {
            let prev = bytes[i - 1];
            let ok_boundary =
                prev.is_ascii_whitespace() || matches!(prev, b'<' | b'"' | b'\'' | b'/' | b'=');
            if !ok_boundary {
                continue;
            }
        }

        let mut j = i + 2;
        let mut had_letter = false;
        while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
            had_letter = true;
            j += 1;
        }
        if !had_letter {
            continue;
        }

        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j < bytes.len() && bytes[j] == b'=' {
            return true;
        }
    }
    false
}

fn sanitize_canvas_html(html: &str, max_bytes: usize) -> anyhow::Result<String> {
    if html.trim().is_empty() {
        bail!("html is empty");
    }
    if html.len() > max_bytes {
        bail!("html too large: {} bytes (max {})", html.len(), max_bytes);
    }

    let lower = html.to_ascii_lowercase();

    for tag in [
        "<script", "</script", "<iframe", "</iframe", "<object", "</object", "<embed", "</embed",
        "<applet", "</applet",
    ] {
        if lower.contains(tag) {
            bail!("forbidden html tag detected: {tag}");
        }
    }

    if contains_event_handler_attr(&lower) {
        bail!("forbidden event handler attribute detected (on* attributes are not allowed)");
    }

    for scheme in ["javascript:", "vbscript:", "data:text/html"] {
        if lower.contains(scheme) {
            bail!("forbidden url scheme detected: {scheme}");
        }
    }

    Ok(html.to_string())
}

fn detect_image_format_and_dimensions(bytes: &[u8]) -> Option<(&'static str, u32, u32)> {
    if let Some((w, h)) = parse_png_dimensions(bytes) {
        return Some(("png", w, h));
    }
    if let Some((w, h)) = parse_jpeg_dimensions(bytes) {
        return Some(("jpeg", w, h));
    }
    if let Some((w, h)) = parse_gif_dimensions(bytes) {
        return Some(("gif", w, h));
    }
    None
}

fn parse_png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() < 24 {
        return None;
    }
    if bytes.get(0..8)? != SIG {
        return None;
    }
    if bytes.get(12..16)? != b"IHDR" {
        return None;
    }

    let w = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
    let h = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
    Some((w, h))
}

fn parse_gif_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 10 {
        return None;
    }
    if bytes.get(0..6)? != b"GIF87a" && bytes.get(0..6)? != b"GIF89a" {
        return None;
    }
    let w = u16::from_le_bytes(bytes.get(6..8)?.try_into().ok()?) as u32;
    let h = u16::from_le_bytes(bytes.get(8..10)?.try_into().ok()?) as u32;
    Some((w, h))
}

fn parse_jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 4 {
        return None;
    }
    if bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return None;
    }

    let mut i = 2usize;
    while i + 1 < bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }

        while i < bytes.len() && bytes[i] == 0xFF {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        let marker = bytes[i];
        i += 1;

        if marker == 0xD9 || marker == 0xDA {
            break;
        }

        if i + 1 >= bytes.len() {
            break;
        }
        let seg_len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
        i += 2;
        if seg_len < 2 || i + seg_len - 2 > bytes.len() {
            break;
        }

        let is_sof = matches!(
            marker,
            0xC0 | 0xC1
                | 0xC2
                | 0xC3
                | 0xC5
                | 0xC6
                | 0xC7
                | 0xC9
                | 0xCA
                | 0xCB
                | 0xCD
                | 0xCE
                | 0xCF
        );
        if is_sof {
            if seg_len < 7 || i + 4 >= bytes.len() {
                return None;
            }
            let h = u16::from_be_bytes([bytes[i + 1], bytes[i + 2]]) as u32;
            let w = u16::from_be_bytes([bytes[i + 3], bytes[i + 4]]) as u32;
            return Some((w, h));
        }

        i += seg_len - 2;
    }

    None
}
