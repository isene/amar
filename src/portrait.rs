//! Portrait generation + storage for PC sheets.
//!
//! Two flows are supported, both bound to the `P` key on a PC:
//!
//! 1. **Clipboard**: build a per-character image-gen prompt, copy it
//!    to the system clipboard (xclip / wl-copy), then ask the user
//!    for the path to the image they got back from ChatGPT. The file
//!    is copied into `~/.amar/campaigns/<c>/portraits/<pc>.png`.
//!
//! 2. **API**: send the same prompt to OpenAI (DALL-E 3) or Google
//!    Gemini (Imagen) based on `GlobalConfig.image_provider`, save
//!    the returned image to the same portraits directory.
//!
//! Either way the saved path is stored on `Character.portrait_path`
//! and rendered by the PC sheet's portrait box.

use crate::pc::Character;
use crate::store::{Campaign, GlobalConfig, campaign_dir};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Build a one-paragraph image-generation prompt from the PC's known
/// fields. Skips any field that's still at its default so a fresh PC
/// with just a name still produces a usable prompt.
pub fn build_prompt(pc: &Character) -> String {
    let mut bits: Vec<String> = Vec::new();
    bits.push("Fantasy RPG character portrait, head-and-shoulders, landscape orientation, painterly digital art".into());
    if !pc.race.is_empty()      { bits.push(format!("a {}", pc.race)); }
    if !pc.gender.is_empty()    { bits.push(match pc.gender.as_str() {
        "M" | "m" | "Male"   | "male"   => "male".into(),
        "F" | "f" | "Female" | "female" => "female".into(),
        other => other.to_string(),
    }); }
    if pc.age > 0               { bits.push(format!("{} years old", pc.age)); }
    if pc.height_cm > 0         { bits.push(format!("{} cm tall", pc.height_cm)); }
    if pc.weight_kg > 0         { bits.push(format!("{} kg", pc.weight_kg)); }
    if !pc.birthplace.is_empty(){ bits.push(format!("from {}", pc.birthplace)); }
    if !pc.description.is_empty() {
        bits.push(pc.description.replace('\n', " "));
    }
    if !pc.clothing.is_empty()  { bits.push(format!("wearing {}", pc.clothing)); }
    let body = bits.join(", ");
    format!("{}. Detailed face, expressive eyes, dramatic lighting, no text, no watermark.",
        body)
}

/// Push `text` onto the system clipboard. Tries `wl-copy` first
/// (Wayland), then `xclip -selection clipboard` (X11). Returns the
/// command name used on success.
pub fn copy_to_clipboard(text: &str) -> Result<&'static str, String> {
    for (cmd, args, name) in &[
        ("wl-copy", &[][..], "wl-copy"),
        ("xclip",   &["-selection", "clipboard"][..], "xclip"),
        ("xsel",    &["--clipboard", "--input"][..],  "xsel"),
    ] {
        match Command::new(cmd).args(*args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(mut child) => {
                if let Some(stdin) = child.stdin.as_mut() {
                    use std::io::Write;
                    if stdin.write_all(text.as_bytes()).is_err() {
                        let _ = child.kill();
                        continue;
                    }
                }
                drop(child.stdin.take());
                if child.wait().map(|s| s.success()).unwrap_or(false) {
                    return Ok(*name);
                }
            }
            Err(_) => continue,
        }
    }
    Err("no clipboard tool found (install wl-copy, xclip, or xsel)".into())
}

/// Where a PC's portrait file lives within a campaign. Sanitises the
/// PC name so spaces / special chars don't break paths.
pub fn portrait_target(camp: &Campaign, pc_name: &str) -> PathBuf {
    let dir = campaign_dir(&camp.name).join("portraits");
    let safe: String = pc_name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    dir.join(format!("{}.png", safe))
}

/// Copy an arbitrary image file into the campaign's portrait dir,
/// returning the destination path. Creates the dir if missing.
pub fn import_image(camp: &Campaign, pc_name: &str, src: &std::path::Path) -> Result<PathBuf, String> {
    let dst = portrait_target(camp, pc_name);
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create dir: {}", e))?;
    }
    std::fs::copy(src, &dst)
        .map_err(|e| format!("copy image: {}", e))?;
    Ok(dst)
}

/// Generate an image via OpenAI DALL-E 3 and write it to an
/// arbitrary path. Used by scene-image generation on the Campaign
/// tab where the destination lives inside an adventure's
/// `Scenes/` folder, not the campaign portraits dir.
pub fn generate_openai_to_path(
    cfg: &GlobalConfig,
    prompt: &str,
    dst: &std::path::Path,
) -> Result<PathBuf, String> {
    let key = read_key(&cfg.openai_key_path)?;
    let body = serde_json::json!({
        "model": "dall-e-3",
        "prompt": prompt,
        "size": "1792x1024",
        "n": 1,
    });
    let resp = ureq::post("https://api.openai.com/v1/images/generations")
        .set("Authorization", &format!("Bearer {}", key))
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| format!("openai request: {}", e))?;
    let json: serde_json::Value = resp.into_json()
        .map_err(|e| format!("openai response: {}", e))?;
    let url = json["data"][0]["url"].as_str()
        .ok_or_else(|| format!("openai response missing url: {}", json))?;
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create dir: {}", e))?;
    }
    let img_resp = ureq::get(url).call()
        .map_err(|e| format!("download image: {}", e))?;
    let mut bytes: Vec<u8> = Vec::new();
    img_resp.into_reader().read_to_end(&mut bytes)
        .map_err(|e| format!("read image bytes: {}", e))?;
    std::fs::write(dst, &bytes).map_err(|e| format!("write image: {}", e))?;
    Ok(dst.to_path_buf())
}

/// Same as `generate_openai_to_path` but via Gemini Imagen.
pub fn generate_gemini_to_path(
    cfg: &GlobalConfig,
    prompt: &str,
    dst: &std::path::Path,
) -> Result<PathBuf, String> {
    let key = read_key(&cfg.gemini_key_path)?;
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/imagen-3.0-generate-001:predict?key={}",
        key);
    let body = serde_json::json!({
        "instances": [{ "prompt": prompt }],
        "parameters": { "sampleCount": 1, "aspectRatio": "16:9" },
    });
    let resp = ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| format!("gemini request: {}", e))?;
    let json: serde_json::Value = resp.into_json()
        .map_err(|e| format!("gemini response: {}", e))?;
    let b64 = json["predictions"][0]["bytesBase64Encoded"].as_str()
        .ok_or_else(|| format!("gemini response missing image: {}", json))?;
    let bytes = base64_decode(b64)
        .map_err(|e| format!("base64 decode: {}", e))?;
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create dir: {}", e))?;
    }
    std::fs::write(dst, bytes).map_err(|e| format!("write image: {}", e))?;
    Ok(dst.to_path_buf())
}

/// Generate an image via OpenAI DALL-E 3 and save it under the
/// campaign's portraits dir. Returns the saved path.
///
/// The API responds with a hosted URL that's valid for an hour;
/// we download the bytes immediately and persist locally so the
/// portrait survives indefinitely.
pub fn generate_openai(
    cfg: &GlobalConfig,
    camp: &Campaign,
    pc_name: &str,
    prompt: &str,
) -> Result<PathBuf, String> {
    let key = read_key(&cfg.openai_key_path)?;
    let body = serde_json::json!({
        "model": "dall-e-3",
        "prompt": prompt,
        "size": "1792x1024",
        "n": 1,
    });
    let resp = ureq::post("https://api.openai.com/v1/images/generations")
        .set("Authorization", &format!("Bearer {}", key))
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| format!("openai request: {}", e))?;
    let json: serde_json::Value = resp.into_json()
        .map_err(|e| format!("openai response: {}", e))?;
    let url = json["data"][0]["url"].as_str()
        .ok_or_else(|| format!("openai response missing url: {}", json))?;
    download_to_portrait(url, camp, pc_name)
}

/// Generate an image via Google Gemini (Imagen 3). Saves to portraits
/// dir, returns the path.
pub fn generate_gemini(
    cfg: &GlobalConfig,
    camp: &Campaign,
    pc_name: &str,
    prompt: &str,
) -> Result<PathBuf, String> {
    let key = read_key(&cfg.gemini_key_path)?;
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/imagen-3.0-generate-001:predict?key={}",
        key);
    let body = serde_json::json!({
        "instances": [{ "prompt": prompt }],
        "parameters": { "sampleCount": 1, "aspectRatio": "16:9" },
    });
    let resp = ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| format!("gemini request: {}", e))?;
    let json: serde_json::Value = resp.into_json()
        .map_err(|e| format!("gemini response: {}", e))?;
    let b64 = json["predictions"][0]["bytesBase64Encoded"].as_str()
        .ok_or_else(|| format!("gemini response missing image: {}", json))?;
    let bytes = base64_decode(b64)
        .map_err(|e| format!("base64 decode: {}", e))?;
    let dst = portrait_target(camp, pc_name);
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create dir: {}", e))?;
    }
    std::fs::write(&dst, bytes).map_err(|e| format!("write image: {}", e))?;
    Ok(dst)
}

fn read_key(path: &str) -> Result<String, String> {
    if path.is_empty() {
        return Err("API key path is unset in config — populate openai_key_path or gemini_key_path".into());
    }
    let s = std::fs::read_to_string(path)
        .map_err(|e| format!("read key {}: {}", path, e))?;
    Ok(s.trim().to_string())
}

fn download_to_portrait(url: &str, camp: &Campaign, pc_name: &str) -> Result<PathBuf, String> {
    let resp = ureq::get(url).call()
        .map_err(|e| format!("download image: {}", e))?;
    let mut bytes: Vec<u8> = Vec::new();
    resp.into_reader().read_to_end(&mut bytes)
        .map_err(|e| format!("read image bytes: {}", e))?;
    let dst = portrait_target(camp, pc_name);
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create dir: {}", e))?;
    }
    std::fs::write(&dst, &bytes).map_err(|e| format!("write image: {}", e))?;
    Ok(dst)
}

/// Tiny standard-base64 decoder. Avoids pulling in the `base64`
/// crate just for one Gemini decode call.
fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    fn val(c: u8) -> Result<u8, String> {
        Ok(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => 0, // padding handled separately
            other => return Err(format!("bad base64 char: {}", other as char)),
        })
    }
    let bytes: Vec<u8> = s.bytes().filter(|&b| !b.is_ascii_whitespace()).collect();
    if bytes.len() % 4 != 0 { return Err("base64 length not multiple of 4".into()); }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        let pad = chunk.iter().filter(|&&b| b == b'=').count();
        let v0 = val(chunk[0])?;
        let v1 = val(chunk[1])?;
        let v2 = val(chunk[2])?;
        let v3 = val(chunk[3])?;
        let n = (v0 as u32) << 18 | (v1 as u32) << 12 | (v2 as u32) << 6 | v3 as u32;
        out.push(((n >> 16) & 0xff) as u8);
        if pad < 2 { out.push(((n >> 8) & 0xff) as u8); }
        if pad < 1 { out.push((n & 0xff) as u8); }
    }
    Ok(out)
}

/// Encode bytes as standard base64 (no line breaks). Used by the
/// kitty-graphics escape that points at the portrait file path.
pub fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() >= 2 { TABLE[((n >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() >= 3 { TABLE[(n & 63) as usize] as char } else { '=' });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip() {
        for v in [
            &b""[..], &b"f"[..], &b"fo"[..], &b"foo"[..],
            &b"foob"[..], &b"fooba"[..], &b"foobar"[..],
        ] {
            let enc = base64_encode(v);
            let dec = base64_decode(&enc).unwrap();
            assert_eq!(dec, v);
        }
    }

    #[test]
    fn prompt_uses_known_fields() {
        let mut c = Character::new_blank("Test");
        c.race = "Halfling".into();
        c.gender = "F".into();
        c.age = 38;
        c.height_cm = 110;
        c.weight_kg = 35;
        c.birthplace = "Riverside".into();
        c.description = "Curly red hair, freckles.".into();
        let p = build_prompt(&c);
        assert!(p.contains("Halfling"));
        assert!(p.contains("female"));
        assert!(p.contains("38 years"));
        assert!(p.contains("110 cm"));
        assert!(p.contains("35 kg"));
        assert!(p.contains("Riverside"));
        assert!(p.contains("Curly red hair"));
    }
}
