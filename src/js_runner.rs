//! JS runner — runs JS files through Falco's TJS engine + HTML script extraction.
//!
//! Usage: falco --js-run <file.js>
//! This evaluates the JS file and prints console.log output.

use crate::tjs::TjsContext;

/// Run a JavaScript file through Falco's TJS engine.
pub fn run_js_file(path: &str) -> Result<(), String> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {path}: {e}"))?;

    // Use interpreter only (VM doesn't support ES2020+ features yet).
    let mut tjs = TjsContext::new();
    tjs.execute(&src)?;
    Ok(())
}

/// Extract all `<script>...</script>` contents from HTML (without the tags).
pub fn extract_scripts(html: &str) -> Vec<String> {
    let mut scripts = Vec::new();
    let lower = html.to_lowercase();
    let mut search_from = 0;
    while let Some(open) = lower[search_from..].find("<script") {
        let open_abs = search_from + open;
        let Some(close_rel) = lower[open_abs..].find('>') else { break };
        let content_start = open_abs + close_rel + 1;
        let Some(end_rel) = lower[content_start..].find("</script>") else { break };
        let content_end = content_start + end_rel;
        let tag_content = &lower[open_abs..open_abs + close_rel];
        if !tag_content.contains("src=") {
            scripts.push(html[content_start..content_end].to_string());
        }
        search_from = content_end + 9;
    }
    scripts
}

/// Extract inline event handlers (onclick, onload, etc.) from an element's
/// attributes. Returns (event_name, handler_source) pairs.
pub fn extract_event_handlers(attrs: &std::collections::HashMap<String, String>) -> Vec<(String, String)> {
    let events = ["onclick", "onload", "onmouseover", "onmouseout", "onsubmit", "onchange", "oninput"];
    let mut handlers = Vec::new();
    for event in events {
        if let Some(handler) = attrs.get(event) {
            handlers.push((event.to_string(), handler.clone()));
        }
    }
    handlers
}
