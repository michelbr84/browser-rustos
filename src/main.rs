//! Browser for rustOS
//!
//! A simple terminal-based web browser that fetches and displays web content.
//! Uses file-based HTTP IPC to communicate with rustOS host.
//!
//! Requires Network permission: `grant browser Network`

use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                    🌐 rustOS Browser v1.0                    ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Default URL to fetch
    let url = "https://httpbin.org/html";

    println!("📍 URL: {}", url);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    match http_get(url) {
        Ok(response) => {
            if let Some(err) = &response.error {
                println!("❌ Error: {}", err);
                return;
            }

            println!("✅ Status: {}", response.status);
            println!("📦 Content Length: {} bytes", response.body.len());
            println!();
            println!("━━━━━━━━━━━━━━━━━━━ Page Content ━━━━━━━━━━━━━━━━━━━");
            println!();

            // Convert HTML to readable text
            let text = html_to_text(&response.body);
            
            // Limit output for terminal readability
            let max_chars = 2000;
            if text.len() > max_chars {
                println!("{}", &text[..max_chars]);
                println!();
                println!("... (truncated, {} more characters)", text.len() - max_chars);
            } else {
                println!("{}", text);
            }
        }
        Err(e) => {
            println!("❌ Request failed: {}", e);
            println!();
            println!("💡 Make sure Network permission is granted:");
            println!("   grant browser Network");
        }
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🏁 Browser session ended.");
}

// ============================================================================
// HTTP Module (file-based IPC with rustOS host)
// ============================================================================

/// HTTP response from a network request
struct HttpResponse {
    status: u16,
    body: String,
    error: Option<String>,
}

/// Perform an HTTP GET request via rustOS file-based IPC
fn http_get(url: &str) -> io::Result<HttpResponse> {
    let net_dir = Path::new("/.net");
    let request_file = net_dir.join("request.json");
    let response_file = net_dir.join("response.json");

    // Check if we already have a response (from a previous run)
    if response_file.exists() {
        let response_content = fs::read_to_string(&response_file)?;
        let _ = fs::remove_file(&response_file); // Clean up
        return parse_response(&response_content);
    }

    // No response yet - write request and exit
    let request_id = generate_request_id();

    // Ensure .net directory exists
    fs::create_dir_all(net_dir)?;

    // Write request as JSON
    let request_json = format!(
        r#"{{"id":"{}","method":"GET","url":"{}","headers":{{}},"body":null}}"#,
        request_id, url
    );

    fs::write(&request_file, request_json)?;

    // Exit the app - rustOS will process the request and re-run us
    println!("[NET] Fetching: {}", url);
    println!("[NET] Waiting for response from rustOS host...");
    std::process::exit(0);
}

fn generate_request_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("req_{}", now.as_millis())
}

fn parse_response(content: &str) -> io::Result<HttpResponse> {
    let status = extract_json_u16(content, "status").unwrap_or(0);
    let body = extract_json_string(content, "body").unwrap_or_default();
    let error = extract_json_string(content, "error");

    Ok(HttpResponse {
        status,
        body,
        error,
    })
}

fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", key);
    if let Some(start) = json.find(&pattern) {
        let value_start = start + pattern.len();
        let remaining = &json[value_start..];

        // Find the closing quote (handling escapes)
        let mut end = 0;
        let mut escaped = false;
        for (i, c) in remaining.chars().enumerate() {
            if escaped {
                escaped = false;
                continue;
            }
            if c == '\\' {
                escaped = true;
                continue;
            }
            if c == '"' {
                end = i;
                break;
            }
        }

        let value = &remaining[..end];
        return Some(unescape_json(value));
    }

    // Check for null value
    let null_pattern = format!("\"{}\":null", key);
    if json.contains(&null_pattern) {
        return None;
    }

    None
}

fn extract_json_u16(json: &str, key: &str) -> Option<u16> {
    let pattern = format!("\"{}\":", key);
    if let Some(start) = json.find(&pattern) {
        let value_start = start + pattern.len();
        let remaining = &json[value_start..];

        // Extract number
        let num_str: String = remaining
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();

        return num_str.parse().ok();
    }
    None
}

fn unescape_json(s: &str) -> String {
    s.replace("\\\"", "\"")
        .replace("\\\\", "\\")
        .replace("\\n", "\n")
        .replace("\\r", "\r")
        .replace("\\t", "\t")
}

// ============================================================================
// HTML to Text Converter
// ============================================================================

/// Convert HTML to readable plain text
fn html_to_text(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let mut tag_name = String::new();
    let mut last_char_was_space = false;

    let chars: Vec<char> = html.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c == '<' {
            in_tag = true;
            tag_name.clear();
            i += 1;
            continue;
        }

        if in_tag {
            if c == '>' {
                in_tag = false;
                let tag_lower = tag_name.to_lowercase();

                // Handle block-level elements
                if tag_lower == "br" || tag_lower == "br/" || tag_lower == "br /" {
                    result.push('\n');
                    last_char_was_space = true;
                } else if matches!(
                    tag_lower.as_str(),
                    "p" | "/p" | "div" | "/div" | "h1" | "/h1" | "h2" | "/h2" | "h3" | "/h3"
                        | "h4" | "/h4" | "h5" | "/h5" | "h6" | "/h6" | "li" | "/li" | "tr" | "/tr"
                ) {
                    if !result.ends_with('\n') {
                        result.push('\n');
                    }
                    last_char_was_space = true;
                }

                // Track script/style tags
                if tag_lower == "script" {
                    in_script = true;
                } else if tag_lower == "/script" {
                    in_script = false;
                } else if tag_lower == "style" {
                    in_style = true;
                } else if tag_lower == "/style" {
                    in_style = false;
                }

                tag_name.clear();
            } else {
                tag_name.push(c);
            }
            i += 1;
            continue;
        }

        // Skip content inside script/style tags
        if in_script || in_style {
            i += 1;
            continue;
        }

        // Handle HTML entities
        if c == '&' {
            let entity_end = chars[i..].iter().position(|&x| x == ';');
            if let Some(end_pos) = entity_end {
                let entity: String = chars[i..i + end_pos + 1].iter().collect();
                let decoded = decode_html_entity(&entity);
                result.push_str(&decoded);
                last_char_was_space = decoded.chars().last().map(|c| c.is_whitespace()).unwrap_or(false);
                i += end_pos + 1;
                continue;
            }
        }

        // Handle whitespace
        if c.is_whitespace() {
            if !last_char_was_space && !result.is_empty() {
                result.push(' ');
                last_char_was_space = true;
            }
        } else {
            result.push(c);
            last_char_was_space = false;
        }

        i += 1;
    }

    // Clean up multiple newlines
    let mut cleaned = String::new();
    let mut prev_was_newline = false;
    for c in result.trim().chars() {
        if c == '\n' {
            if !prev_was_newline {
                cleaned.push(c);
            }
            prev_was_newline = true;
        } else {
            cleaned.push(c);
            prev_was_newline = false;
        }
    }

    cleaned
}

/// Decode common HTML entities
fn decode_html_entity(entity: &str) -> String {
    match entity {
        "&nbsp;" => " ".to_string(),
        "&lt;" => "<".to_string(),
        "&gt;" => ">".to_string(),
        "&amp;" => "&".to_string(),
        "&quot;" => "\"".to_string(),
        "&apos;" => "'".to_string(),
        "&copy;" => "©".to_string(),
        "&reg;" => "®".to_string(),
        "&trade;" => "™".to_string(),
        "&mdash;" => "—".to_string(),
        "&ndash;" => "–".to_string(),
        "&hellip;" => "…".to_string(),
        "&bull;" => "•".to_string(),
        _ => {
            // Try to parse numeric entities like &#39; or &#x27;
            if entity.starts_with("&#x") && entity.ends_with(';') {
                let hex_str = &entity[3..entity.len() - 1];
                if let Ok(code) = u32::from_str_radix(hex_str, 16) {
                    if let Some(c) = char::from_u32(code) {
                        return c.to_string();
                    }
                }
            } else if entity.starts_with("&#") && entity.ends_with(';') {
                let num_str = &entity[2..entity.len() - 1];
                if let Ok(code) = num_str.parse::<u32>() {
                    if let Some(c) = char::from_u32(code) {
                        return c.to_string();
                    }
                }
            }
            entity.to_string()
        }
    }
}
