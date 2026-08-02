//! Falco — a tiny, from-scratch browser engine written in Rust.
//!
//! Pipeline:
//!   HTML source → DOM → (CSS source → Stylesheet) → Style tree
//!   → Layout tree → Paint commands → Canvas → PNG
//!
//! ```no_run
//! use falco::{render_to_png, RenderOptions};
//! let opts = RenderOptions { width: 1200, height: 800, ..Default::default() };
//! render_to_png("<h1>Hello</h1>", "h1 { color: red; }", opts, "out.png").unwrap();
//! ```
//!
//! ## Module status (honest)
//!
//! The render pipeline currently uses the *legacy* modules below. They are
//! the ones actually called by [`render_with_base_url`]:
//!   - [`html`] — HTML parser (legacy)
//!   - [`dom`] — DOM types (legacy)
//!   - [`css`] — CSS parser + selector matching (legacy)
//!   - [`style`] — cascade + UA styles
//!   - [`layout`] — block/inline/flex/grid/table/float/absolute layout
//!   - [`paint`] — canvas + fonts + compositing
//!   - [`svg`] — SVG parser + renderer
//!   - [`png`] — hand-written PNG encoder
//!   - [`image`] — image loader (HTTP, data: URLs, local files)
//!   - [`js_tjs`], [`js_runner`] — JS execution via the TJS VM
//!   - [`tjs`] — the JavaScript VM itself
//!   - [`window`] — interactive `--window` mode
//!   - [`net`] — HTTP fetch, cookies, cache, websocket, redirect
//!
//! The following modules are **structurally complete and pass their own
//! unit tests, but NOT YET wired into the render pipeline**. They exist as
//! spec-compliant replacements for the legacy types above and are the
//! planned v0.2.0 milestone. Until then they are exported for review and
//! experimentation, but `render_with_base_url` does not call into them:
//!   - [`html::spec`] — WHATWG §13.2 tokenizer + tree builder + serializer + XML
//!   - [`dom::spec`] — spec DOM, MutationObserver, Shadow DOM, custom elements, a11y
//!   - [`css::spec`] — Selectors Level 4, cascade specificity, @-rules
//!   - [`tjs_ext`] — Symbol, BigInt, Promise, Map/Set, WeakMap/WeakSet, Reflect
//!
//! The following modules are **implemented but not enforced in the
//! renderer** — same situation, planned for a later milestone:
//!   - [`security`] — SOP, multi-process, sandbox, CSP, certs, permissions
//!   - [`web_runtime`] — fetch, XHR, event loop, WebGL/video/MSE/EME/NDSD stubs
//!   - [`media`] — media queries (parsed but always applied)

// === Render pipeline (active) ===
// dom/, css/, html/ each contain:
//   - the legacy (currently active) types at the module root
//   - the spec-compliant replacement under `spec/` (WIP, not yet wired in)
pub mod css;
pub mod dom;
pub mod html;
pub mod image;
pub mod js_runner;
pub mod js_tjs;
pub mod layout;
pub mod media;
pub mod net;
pub mod paint;
pub mod png;
pub mod style;
pub mod svg;
pub mod tjs;
pub mod window;

// === Spec-compliant runtime extensions (NOT yet wired into render pipeline) ===
// These extend the active `tjs/` VM with Symbol, BigInt, Promise, etc.
// They compile and pass their own unit tests, but the JS bindings exposed
// to `onclick=` handlers don't yet surface all of these to user scripts.
pub mod tjs_ext;

// === Security / web runtime (implemented but not enforced in renderer) ===
pub mod security;
pub mod web_runtime;

use paint::{Canvas, FontRasterizer, PaintCommand};

/// Extract the contents of all `<style>...</style>` tags from HTML.
fn extract_style_tags(html: &str) -> String {
    let mut result = String::new();
    let lower = html.to_lowercase();
    let mut pos = 0;
    while pos < html.len() {
        if let Some(start) = lower[pos..].find("<style") {
            let abs_start = pos + start;
            // Find the end of the opening <style ...> tag.
            if let Some(tag_end) = lower[abs_start..].find('>') {
                let content_start = abs_start + tag_end + 1;
                // Find </style>.
                if let Some(close) = lower[content_start..].find("</style>") {
                    let content_end = content_start + close;
                    result.push_str(&html[content_start..content_end]);
                    result.push('\n');
                    pos = content_end + 8; // skip past </style>
                    continue;
                }
            }
            pos = abs_start + 1;
        } else {
            break;
        }
    }
    result
}

/// Strip all <script>...</script> tags from HTML, replacing with nothing.
fn strip_scripts(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let lower = html.to_lowercase();
    let mut pos = 0;
    while pos < html.len() {
        if let Some(start) = lower[pos..].find("<script") {
            let abs_start = pos + start;
            result.push_str(&html[pos..abs_start]);
            if let Some(end) = lower[abs_start..].find("</script>") {
                pos = abs_start + end + 9;
            } else {
                if let Some(close) = lower[abs_start..].find("/>") {
                    pos = abs_start + close + 2;
                } else {
                    pos = html.len();
                }
            }
        } else {
            result.push_str(&html[pos..]);
            break;
        }
    }
    result
}

/// Inject minimal fallback CSS — only ensures hidden elements become visible.
fn inject_fallback_css(html: &str) -> String {
    let fallback = r#"<style>
        [style*="display:none"], [style*="display: none"] { display: block !important; }
        [hidden] { display: block !important; }
        svg { display: none !important; }
    </style>"#;
    let lower = html.to_lowercase();
    if let Some(pos) = lower.rfind("</head>") {
        let mut result = String::with_capacity(html.len() + fallback.len());
        result.push_str(&html[..pos]);
        result.push_str(fallback);
        result.push_str(&html[pos..]);
        result
    } else {
        format!("{}{}", fallback, html)
    }
}

/// Options for a single render pass.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Viewport width in pixels.
    pub width: u32,
    /// Viewport height cap in pixels. The canvas grows if content is taller.
    pub height: u32,
    /// Background color (RGBA hex like 0xFFFFFFFF for white).
    pub background: u32,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: 1200,
            height: 800,
            background: 0xFFFFFFFF,
        }
    }
}

/// Render HTML + CSS to a PNG file. Convenience entry point for the library.
pub fn render_to_png(
    html_src: &str,
    css_src: &str,
    opts: RenderOptions,
    output_path: &str,
) -> anyhow::Result<()> {
    let (canvas, _commands) = render(html_src, css_src, opts)?;
    let png_bytes = png::encode(canvas.width, canvas.height, &canvas.pixels);
    std::fs::write(output_path, png_bytes)?;
    Ok(())
}

/// Render HTML + CSS to a Canvas + the list of paint commands (for inspection).
pub fn render(
    html_src: &str,
    css_src: &str,
    opts: RenderOptions,
) -> anyhow::Result<(Canvas, Vec<PaintCommand>)> {
    render_with_base_url(html_src, css_src, opts, "")
}

/// Render HTML + CSS with a base URL (for resolving relative image URLs).
///
/// This function:
/// 1. Parses HTML into a dom2 document.
/// 2. Executes inline `<script>` tags via TJS with DOM bridge.
/// 3. Serializes the (possibly modified) DOM back to HTML.
/// 4. Parses CSS.
/// 5. Builds style tree → layout tree → paint commands.
/// 6. Paints to canvas.
pub fn render_with_base_url(
    html_src: &str,
    css_src: &str,
    opts: RenderOptions,
    base_url: &str,
) -> anyhow::Result<(Canvas, Vec<PaintCommand>)> {
    // Set the base URL for image resolution.
    image::set_base_url(base_url);

    // 1. Extract and execute inline <script> tags via TJS.
    let scripts = crate::js_runner::extract_scripts(html_src);
    if !scripts.is_empty() {
        let mut tjs = crate::tjs::TjsContext::new();
        for script in &scripts {
            if let Err(e) = tjs.execute(script) {
                eprintln!("[falco:tjs] script error (continuing): {}", e);
            }
        }
    }
    // 2. Strip <script> tags and inject fallback CSS to ensure visibility.
    let clean_html = strip_scripts(html_src);
    let clean_html = inject_fallback_css(&clean_html);
    // 3. Extract <style> tags from HTML and merge with external CSS.
    let all_css = format!("{}\n{}", extract_style_tags(&clean_html), css_src);
    // 4. Parse the cleaned HTML.
    let dom = html::parse(&clean_html);
    // 5. Parse CSS (merged from <style> tags + external).
    let stylesheet = css::parse(&all_css);
    // 6. Build style tree.
    let style_tree = style::build_style_tree(&dom, &stylesheet);
    // 7. Build layout tree.
    let layout_root = layout::build_layout_tree(&style_tree, opts.width as f32);
    // 8. Collect paint commands.
    let mut commands = Vec::new();
    layout::collect_paint_commands(&layout_root, &mut commands);
    // 9. Determine final canvas height.
    let actual_height = (layout_root.bounds.y + layout_root.bounds.height).ceil() as u32;
    let canvas_height = actual_height.max(opts.height);
    // 10. Paint.
    let mut canvas = Canvas::new(opts.width, canvas_height);
    let bg = opts.background;
    let bg_color = css::Color::rgba(
        ((bg >> 24) & 0xff) as u8,
        ((bg >> 16) & 0xff) as u8,
        ((bg >> 8) & 0xff) as u8,
        (bg & 0xff) as u8,
    );
    if bg_color.a > 0 {
        canvas.fill_rect(0.0, 0.0, opts.width as f32, canvas_height as f32, bg_color);
    }
    let rasterizer = FontRasterizer::new()?;
    paint::paint(&commands, &mut canvas, &rasterizer);
    Ok((canvas, commands))
}
