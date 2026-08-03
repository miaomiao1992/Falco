<p align="center">
  <img src="docs/falco.svg" alt="Falco logo" width="220" height="220">
</p>

<h1 align="center">Falco</h1>

<p align="center">
  <em>Logo by <strong>Sanya</strong> — <a href="https://t.me/SanyochekDev">t.me/SanyochekDev</a></em>
</p>

<p align="center">
  A tiny, fast browser engine written in Rust.<br>
  Renders HTML, CSS, JavaScript, SVG and images — to a PNG, or to a live interactive window.
</p>

<p align="center">
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-yellow.svg"></a>
  <img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-rust-orange.svg">
  <img alt="Binary size" src="https://img.shields.io/badge/binary-~10%20MB-blue.svg">
  <img alt="Lines of Rust" src="https://img.shields.io/badge/LOC-~36%2C000-9cf.svg">
  <a href="https://github.com/poxk/Falco/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/poxk/Falco/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://send.monobank.ua/jar/21T9ZRwZq3"><img alt="Donate" src="https://img.shields.io/badge/donate-MonoBank-7c3aed.svg"></a>
</p>

<p align="center">
  <a href="#quick-start">Quick start</a> ·
  <a href="#interactive-mode---window">Interactive mode</a> ·
  <a href="#what-it-supports">Features</a> ·
  <a href="#architecture">Architecture</a> ·
  <a href=".github/CONTRIBUTING.md">Contributing</a>
</p>

---

## What it is

Falco is a real browser engine in roughly **36,000 lines of Rust**. It
parses HTML, applies CSS, executes JavaScript, loads images, computes
layout, and paints to a canvas — either as a **PNG file** or a live
**interactive window** where you can scroll, click links, fill out
forms, and navigate.

```
HTML ──▶ DOM ──▶ Style tree ──▶ Layout tree ──▶ Paint commands ──▶ Canvas ──▶ PNG / Window
            ▲           ▲              ▲
            │           │              │
       HTML5 tokenizer  CSS cascade    Flex / Grid / Table / Float / Absolute
       + tree builder   + Selectors 4  + Inline / Block flow
            │
       JS (custom VM with closures, generators, Promise, BigInt, Symbol)
            │
       Image loader ──▶ HTTP / data: URL / local file
```

Falco is **not** a wrapper around WebKit, Gecko, or Chromium. Every
module — HTML tokenizer, CSS parser, layout engine, JS VM, font
rasterizer, PNG encoder — is written from scratch in Rust.

## Honest status (what works vs what's a stub)

To set expectations clearly — this is v0.1.0, an early release by a
single developer. Not everything listed in this README is production-ready.
Here's what is actually wired into the render pipeline and what is
structurally complete but not yet called:

### ✅ Actually works (called by `render_with_base_url`)
- HTML parser (`html/`) — legacy parser, not spec-compliant but functional
- DOM types (`dom/`) — minimal node types, no observers/shadow
- CSS parser (`css/`) — selectors, properties, cascade, color parsing
- Style cascade (`style/`) — UA styles + inheritance + flex/grid props
- Layout (`layout/`) — block / inline / flex / **CSS Grid** / **table** / float / absolute
- Painting (`paint/`) — fonts (ab_glyph), gradients, shadows, alpha compositing
- SVG renderer (`svg/`) — paths, basic shapes, gradients, stroke + fill
- Hand-written PNG encoder (`png/`)
- Image loader (`image/`) — HTTP, data: URLs, local files
- JS VM (`tjs/`) — bytecode interpreter + JIT (x86_64, Linux-only)
- JS-DOM bindings (`js_tjs/`, `js_runner/`) — `document.getElementById`, `console.log`, `alert`, `onclick`
- **DOM mutation from JS** (new in v0.2.0) — `element.innerHTML = ...`, `element.style.color = ...`, `appendChild`, `removeChild` all mutate the spec DOM and trigger re-render via the `legacy_dom_to_spec_document` → `TjsJsContext` → `serialize_spec_document` bridge
- **Real `fetch()` from JS** (new in v0.2.0) — blocking HTTP via `ureq`, returns response with `ok`, `status`, `text()`, `json()`
- Networking (`net/`) — HTTP/1.1 (ureq), cookies, cache, websocket, redirect
- Interactive `--window` mode — scrolling, forms, navigation, history

### ⚠️ Structurally complete, passes own unit tests, **NOT wired into renderer**
These exist as spec-compliant replacements for the legacy modules. They
compile and have their own unit tests, but `render_with_base_url` does
not call into them yet. This is the v0.3.0 milestone.
- `html::spec` — WHATWG §13.2 tokenizer (all 80 states) + tree builder (all 22 insertion modes) + serializer + XML parser + encoding detection. **Note**: the serializer IS used by the v0.2.0 DOM mutation bridge, but the tokenizer/tree_builder are not.
- `dom::spec` — spec DOM with MutationObserver, Shadow DOM, custom elements, accessibility tree. **Note**: the core `Node`/`ElementData`/`Document` types ARE used by the v0.2.0 DOM mutation bridge, but MutationObserver/Shadow DOM/custom elements are not yet exposed to JS.
- `css::spec` — Selectors Level 4 (`:has()`, `:is()`, `:where()`, cascade layers, container queries)
- `tjs_ext/` — Symbol, BigInt, Promise, microtasks, Map/Set, WeakMap/WeakSet, Reflect

### ⚠️ Implemented algorithms, **NOT enforced in renderer**
- `security/` — SOP, multi-process, seccomp sandbox, CSP, TLS cert chain validation, permissions, extensions, DevTools protocol. All algorithms are in place and unit-tested, but the renderer doesn't enforce them yet.
- `web_runtime/` — `fetch()` is **real** as of v0.2.0 (uses `ureq` for blocking HTTP, returns response with `text()` and `json()`). `XMLHttpRequest` is still a stub. The Promise / event loop integration is real and unit-tested, but the JS bridge doesn't yet integrate with it (fetch is blocking, not async/Promise-based). WebGL, video, MSE, EME, NDSD are **headless stubs** — they implement the API surface but don't actually render WebGL frames, decode H.264 video, or do DRM. They exist as scaffolding for future work.
- `media/` — `@media` queries are parsed but always applied (no conditional cascade yet)

### ❌ Known broken / unfinished
- `real-http2`, `real-webgl`, `sandbox` Cargo features do not compile with `--all-features` because their upstream APIs have drifted (h2::Body removed, glow API changed, seccomp pre_exec is Unix-only). They are disabled by default and the CI clippy step does not check them.
- The JIT (`tjs/jit.rs`) works on Linux x86_64 but fails on macOS CI runners because `mmap(MAP_JIT)` requires code signing with the `com.apple.security.cs.allow-jit` entitlement. JIT tests are marked `#[ignore]` on macOS.
- HTML5 spec-compliant tree repair (adoption agency, foster parenting) is in `html/spec/tree_builder.rs` but the legacy parser is what actually runs.

**Bottom line:** if you `cargo build && ./falco https://example.com --out out.png`, you get a real PNG render. The HTML/CSS/layout/paint path works end-to-end. The spec-compliant parsers, security enforcement, and advanced web runtime (WebGL/video/DRM) are **scaffolding for future milestones**, not working features. Please read the code before claiming otherwise.

## Quick start

```bash
# Build (release binary lands in target/release/falco)
cargo build --release

# Render a local HTML file to PNG
./target/release/falco page.html --out page.png --width 1200

# Render a URL to PNG
./target/release/falco https://example.com --out example.png --width 800

# Open interactive live window (desktop only — scrolls, hover, link clicks)
./target/release/falco page.html --window

# Render with external CSS merged on top of <style> tags
./target/release/falco README.html --css style.css --out readme.png
```

`<style>` tags inside the HTML are automatically extracted and
applied. External CSS via `--css` is merged on top.

## Interactive mode (`--window`)

When you pass `--window`, Falco opens a real browser window with an
**address bar at the top**, the **page content in the middle**, and a
**status bar at the bottom**. You can:

### Navigation
- **Click links** (`<a href>`) — Falco fetches the new URL and re-renders
- **Address bar** — click it or press `F6`, type a URL, press `Enter`
- **Reload** with `r` — re-fetches the current page
- **Back/Forward** with `Alt+←` / `Alt+→` — full history navigation

### Forms
- **Click inputs** to focus them, then **type** to enter text (live update)
- **Tab / Shift+Tab** — cycle focus between interactive elements
- **Backspace** — delete the last character
- **Enter** — submit form / click the focused button / follow focused link
- Supported types: `text`, `email`, `password` (masked), `checkbox`, `submit`, `button`, `textarea`

### Scrolling & visual feedback
- **Mouse wheel**, **arrow keys**, **Page Up/Down**, **Home/End**
- **Focus ring** — 2px blue outline around the focused element
- **Blinking caret** — 500ms blink in text inputs and the address bar
- **Hover hint** — when hovering a link, its href appears in the status bar

### Incremental repaint
The window only re-paints when state actually changes (scroll, input
text, focus, hover). On idle frames, only the address bar / status bar
overlays are redrawn — the page canvas is cached. This keeps CPU usage
low and scrolling smooth.

### Headless fallback
If no display is available (headless server, no X11/Wayland), Falco
automatically falls back to PNG output with a warning.

## CLI reference

```
falco — a tiny browser engine

USAGE
  falco <input> [OPTIONS]

INPUT
  A URL (http://...) or a path to a local .html file.

OPTIONS
  --css <path>       External CSS file (merged with <style> tags in HTML).
  --out <path>       Output PNG path. Default: falco.png
  --width <px>       Viewport width. Default: 1200
  --height <px>      Viewport height (canvas grows if content is taller). Default: 800
  --bg <hex>         Background color (0xRRGGBBAA). Default: 0xFFFFFFFF
  --window           Open a live interactive window instead of writing PNG.
  -h, --help         Show this help
  -V, --version      Print version

INTERACTIVE MODE KEYS (when --window is used)
  q / Esc       quit
  r             reload (prints message — restart Falco to actually reload)
  ↑ / ↓         scroll line
  PgUp / PgDn   scroll page
  Home / End    jump to top / bottom
  g / G         top / bottom (vim-style)
  mouse wheel   scroll
  mouse click   follow `<a href>` link (prints URL to stderr)
```

## Programmatic API

### Render to PNG

```rust
use falco::{render_to_png, RenderOptions};

fn main() -> anyhow::Result<()> {
    let html = std::fs::read_to_string("page.html")?;
    let css = std::fs::read_to_string("style.css")?;
    let opts = RenderOptions {
        width: 1200,
        height: 800,
        background: 0xFFFFFFFF,
    };
    render_to_png(&html, &css, opts, "out.png")?;
    Ok(())
}
```

### Render to raw RGBA buffer (for game engines / GPU textures)

New in v0.1.1. Skips PNG encoding and returns raw RGBA pixels, ready to
upload to a GPU texture.

```rust
use falco::{render_to_buffer, RenderOptions};

fn main() -> anyhow::Result<()> {
    let opts = RenderOptions { width: 1280, height: 720, ..Default::default() };
    let rendered = render_to_buffer("<h1>Hello</h1>", "h1 { color: red; }", opts)?;

    // RGBA, row-major, top-to-bottom:
    let rgba: &[u8] = rendered.as_rgba();

    // For DirectX / Win32 / Vulkan surfaces that expect BGRA:
    let bgra: Vec<u8> = rendered.to_bgra();

    // For RGB-only contexts (no alpha):
    let rgb: Vec<u8> = rendered.to_rgb();

    Ok(())
}
```

This is the intended entry point for embedding Falco into game engines
and GUI toolkits (Bevy, Fyrox, egui, custom engines). The `RenderedBuffer`
struct also exposes `width` and `height` so you can size your GPU texture
correctly.

## Benchmarks

Measured on Linux, AMD Ryzen 5 5600X, release build. Times include
HTML parse + CSS cascade + layout + paint + PNG encode.

| Page | HTML size | Render time | Output PNG |
|------|-----------|-------------|------------|
| `https://example.com` | 1.1 KB | ~46 ms (incl. network fetch) | 800x600 |
| `tests/fixtures/modern.html` (flexbox + gradients) | 3.5 KB | ~110 ms | 1200x998 |
| `tests/fixtures/sample.html` | 2.5 KB | ~116 ms | 1200x2471 |
| Hacker News front page | ~50 KB | ~300 ms | 1280x1440 |
| Simple `<h1>Hello</h1>` (no network) | 30 B | ~12 ms | 1200x60 |

Cold start (empty image cache) adds ~50 ms on first render. Warm cache
is what the table above shows.

Binary size: ~10 MB (release, stripped, with default features).

Proper `criterion` benchmarks are on the v0.1.2 todo list.

## What it supports

### HTML
- **HTML5 tokenizer** (WHATWG §13.2.5) — all 80 states, script-data
  escape/double-escape, attribute parsing with duplicate detection,
  named + numeric character references with Windows-1252 quirks
- **HTML5 tree builder** (WHATWG §13.2.6) — all 22 insertion modes,
  stack of open elements with scope algorithms, active formatting
  elements list, reconstruct active formatting, **adoption agency
  algorithm**, **foster parenting** for table content, `<template>`
  with separate DocumentFragment contents
- **Entity references** — `&amp;`, `&#65;`, `&copy;`, 50+ named entities
- **Whitespace collapsing** (browser-style)
- **`<template>` element** with separate DocumentFragment contents
- **XML/XHTML parser** — strict, with namespace bindings, CDATA, PIs
- **Encoding detection** — BOM, HTTP `Content-Type` charset, `<meta
  charset>`, `<meta http-equiv>`, heuristic UTF-8/UTF-16 detection,
  decoders for UTF-8 / UTF-16LE / UTF-16BE / Windows-1252
- **Tree builder auto-close** — `<li>`, `<p>`, `<td>`, `<tr>`,
  `<option>`, `<dt>`/`<dd>`
- **innerHTML / outerHTML serialization** — void elements, `<template>`
  contents fragment, raw text elements, full attribute value escaping

### DOM (spec-compliant, `dom::spec`)
- `NodeRef = Rc<RefCell<Node>>` with `parent`, `firstChild`,
  `lastChild`, `previousSibling`, `nextSibling` pointers per spec
- `DocumentHandle = Rc<RefCell<Document>>` with weak back-ref from
  each Node
- **Mutation records** queued on every
  append/insert/remove/setAttribute/removeAttribute
- **MutationObserver** with `observe()`, `disconnect()`,
  `take_records()`, `MutationObserverInit`
  (childList/attributes/characterData/subtree/attributeOldValue/
  characterDataOldValue/attributeFilter), subtree ancestor matching
- **Shadow DOM** — `attachShadow()` with open/closed modes, host
  validation, named + default slots, fallback content, slot
  distribution (flatten tree algorithm), `assignedSlot` lookup
- **Custom elements** — `customElements.define()` with name validation,
  `observedAttributes` tracking, lifecycle callbacks (connected /
  disconnected / adopted / attributeChanged / form-associated),
  pending upgrades, customized built-in elements (`is="..."`)
- **Accessibility tree** — parallel tree with role/name/description/
  state/actions, implicit ARIA roles for ~50 HTML tags, honors
  `role=""`, `aria-hidden`, `hidden`, `display:none`, accessible name
  computation (aria-label > aria-labelledby > element-specific > title)

### CSS (`css/` + `css::spec`)
- **Selectors Level 4** — type, class, id, universal (`*`), descendant,
  child (`>`), adjacent sibling (`+`), general sibling (`~`),
  attribute (`[attr]`, `=`, `~=`, `|=`, `^=`, `$=`, `*=`)
- **Pseudo-classes** — `:hover`, `:focus`, `:focus-visible`,
  `:focus-within`, `:active`, `:visited`, `:checked`, `:disabled`,
  `:enabled`, `:readonly`, `:readwrite`, `:required`, `:optional`,
  `:valid`, `:invalid`, `:empty`, `:root`, `:first-child`,
  `:last-child`, `:only-child`, `:first-of-type`, `:last-of-type`,
  `:only-of-type`, `:nth-child(an+b)`, `:nth-last-child`,
  `:nth-of-type`, `:nth-last-of-type`, `:nth-child(an+b of S)`,
  `:is()`, `:where()`, `:not()`, `:has()`, `:lang()`, `:dir()`
- **Cascade & specificity** — (a, b, c) tuple, `:where()` zero,
  `:is()` / `:not()` / `:has()` most specific arg, CascadeOrigin
  (UA / User / Author) with reversed order for `!important`,
  CascadeLayers (None wins over layered; later wins over earlier)
- **Properties** — `display`, `position`, `color`, `background`
  (including `linear-gradient` and `radial-gradient`), `font-*`,
  `margin`, `padding`, `border`, `border-radius`, `width`, `height`,
  `min/max-width`, `top/right/bottom/left`, `z-index`, `overflow`,
  `opacity`, `box-shadow`, `white-space`, `box-sizing`, `gap`,
  `flex*`, `grid*`, `writing-mode`, logical properties
  (`margin-inline-start`, etc.)
- **Values** — keywords, hex/rgb/rgba/named colors, lengths (px, em,
  rem, pt, %, vw, vh), percentages, numbers, `!important`
- **Functions** — `linear-gradient()`, `radial-gradient()`, `url()`,
  `var()`, `calc()` (simplified), `rgb()`, `rgba()`
- **Shorthands** — `margin`, `padding`, `border`, `background`, `flex`
- **@-rules** — `@media` (parsed and applied), `@keyframes` /
  `@animation` with cubic-bezier & steps timing functions,
  `@font-face` with family/weight lookup, `@layer` (cascade layers),
  `@container` queries with `evaluate_container_query()`
- **Logical properties** — `margin-inline-start` etc. resolved to
  physical properties based on `writing-mode`
- **CSS counters** — `counter-reset`, `counter-increment`, `counter-set`
- **Containment** — `contain: layout/paint/size/style/inline-size/
  block-size`, `strict`, `content`
- **Filters** — `blur`, `brightness`, `contrast`, `drop-shadow`,
  `grayscale`, `hue-rotate`, `invert`, `opacity`, `saturate`, `sepia`
- **Clip-path** — `polygon`, `circle`, `ellipse`, `inset`, `path`,
  `url()`

### Layout (`layout/`)
- **Block flow** — vertical stacking
- **Inline flow** — horizontal text wrapping with proper baseline
- **Flexbox** — `flex-direction`, `justify-content`, `align-items`,
  `flex-wrap`, `gap`, `flex-grow`, `flex-shrink`, `flex-basis`
- **CSS Grid** — `grid-template-columns` / `grid-template-rows` (with
  `fr`, `auto`, `minmax()`, `repeat()`), `grid-column` /
  `grid-row` placement, `gap` / `column-gap` / `row-gap`, `auto-flow`
- **Table layout** — `<table>`, `<tr>`, `<td>`, `<th>`, `<thead>`,
  `<tbody>`, `<tfoot>`, `<caption>`, column width distribution,
  border collapse
- **Float** — `float: left/right`, simple clear
- **Inline-block** — inline elements with block-like width/height
- **Box model** — margin, border, padding, content with
  `box-sizing: border-box` support
- **Position** — `static`, `relative`, `absolute`, `fixed` (parsed,
  relative + absolute positioning applied)
- **Units** — px, em, rem, pt, %, vw, vh
- **Writing modes** — `horizontal-tb`, `vertical-rl/lr`,
  `sideways-rl/lr`

### Painting (`paint/`)
- **Backgrounds** — solid colors and linear / radial gradients
- **Borders** — all four sides with custom colors and styles
- **Border-radius** — rounded corners (all four corners)
- **Box-shadow** — outer shadows with blur
- **Opacity** — alpha blending for entire elements
- **Text** — TrueType font rasterization via `ab_glyph`, bold and
  italic synthesis
- **Alpha compositing** — proper RGBA blending
- **SVG** — paths, basic shapes (`rect`, `circle`, `ellipse`,
  `line`, `polyline`, `polygon`), gradients, stroke + fill
- **PNG encoder** — hand-written, no `flate2` dependency

### JavaScript (`tjs/` + `tjs_ext/`)

Falco ships its own JavaScript VM (the `tjs` module) — pure Rust, no
V8/SpiderMonkey/`boa`. It is a bytecode VM with a generational GC,
inline caching, hidden classes, and a JIT tier-up.

- **ES2015+ syntax** — `let` / `const`, arrow functions, template
  literals, destructuring (object + array), default + rest params,
  spread, `for...of`, `for...in`, computed property names, shorthand
  methods/properties, optional chaining, nullish coalescing,
  exponentiation operator, async/await (parsed)
- **Functions** — `function foo() {}`, closures with proper upvalue
  capture, generators (`function*` / `yield` / `yield*`),
  `async function` / `await` (parser-level)
- **Types** — `Symbol` with 13 well-known symbols, `BigInt` with
  arbitrary precision (u32 limbs, signed), `Promise` with
  `then`/`catch`/`finally` + state machine, `Iterator` protocol,
  `Generator` as state machine
- **Built-ins** — `Math`, `JSON`, `Array` (`push`, `pop`, `map`,
  `filter`, `reduce`, `forEach`, `find`, `findIndex`, `includes`,
  `slice`, `splice`, `flat`, `flatMap`), `String` (`split`, `replace`,
  `match`, `padStart`, `padEnd`, `trim`, `trimStart`, `trimEnd`,
  `startsWith`, `endsWith`, `includes`, `repeat`), `Object` (`keys`,
  `values`, `entries`, `assign`, `freeze`, `fromEntries`),
  `Reflect` (`get`, `set`, `has`, `deleteProperty`, `ownKeys`),
  `WeakMap`, `WeakSet`, `Map`, `Set`, `Proxy`
- **Microtask queue** — `Promise` reactions drained as microtasks
- **DOM bindings** — `document.getElementById`,
  `document.querySelector` / `querySelectorAll`, `console.log`,
  `alert`, `addEventListener` (basic)
- **Event loop integration** — `setTimeout`, `setInterval`,
  `requestAnimationFrame`, `fetch()` (returns `Promise<Response>`),
  `XMLHttpRequest`, all driven by the event loop in
  `web_runtime/event_loop.rs`
- **console.log(...)** — prints to stderr
- **alert(msg)** — shows in the status bar

### Networking (`net/`)
- HTTP/1.1 fetch (via `ureq`)
- HTTP/2 parser (`http2.rs`)
- Cookie jar (`cookies.rs`) with proper domain/path matching
- Redirect handling (`redirect.rs`) with redirect-loop detection
- Cache (`cache.rs`) — HTTP cache with conditional requests
- WebSocket (`websocket.rs`) — frame parser, masking, ping/pong

### Web runtime (`web_runtime/`)
- `fetch()` (`fetch.rs`) — Promise-based, integrates with event loop
- `XMLHttpRequest` (`xhr.rs`) — sync + async modes
- Event loop (`event_loop.rs`) — task queues, microtasks, RAF
- `Promise` (`promise.rs`) — state machine, then/catch/finally
- WebGL (`webgl.rs`) — shader compilation, buffer management, draw
  calls (headless)
- Video (`video.rs`) — `<video>` element demux + decode stub
- MSE (`mse.rs`) — Media Source Extensions
- EME (`eme.rs`) — Encrypted Media Extensions
- NDSD (`ndsd.rs`) — Native Device Service Discovery

### Security (`security/`) — implemented but not fully wired into renderer
- **Origin / SOP** (`origin.rs`) — Origin struct, `is_same_origin`,
  `is_same_site`, `registrable_domain` (with 2-part TLD list),
  `check_cors` (with credentials / wildcard handling),
  `check_navigation`
- **Multi-process / site isolation** (`process.rs`) — Process kinds
  (Browser / Renderer / GPU / Utility / Plugin), site-to-process
  map, ProcessPerSite / ProcessPerTab policies, crash recovery with
  max-restarts / sad-tab / fatal modes
- **Sandbox** (`sandbox.rs`) — seccomp-bpf filter (Linux), renderer
  allowlist (~30 syscalls), blocks `execve`/`fork`/`ptrace`/`open`/
  `socket`/`connect`/`mount`, `PR_SET_NO_NEW_PRIVS`, `drop_capabilities`
- **CSP** (`csp.rs`) — directive map, `default-src` fallback, source
  expression matching (`'self'`, `'none'`, `'unsafe-inline'`,
  `'unsafe-eval'`, `data:`, `blob:`, host, `*.wildcard`, `scheme:`),
  nonce/hash support, `allows_inline_script`, `allows_eval`,
  `allows_javascript_url`, `is_safe_attribute` (blocks `onclick`,
  `onerror`, `javascript:` in href/src), violation reports
- **TLS certificates** (`cert.rs`) — Certificate struct, validity,
  hostname matching (with wildcards), TrustStore with Mozilla
  defaults, `validate_chain` (chain building, signature check,
  hostname, EKU, path length), OCSP stub, HPKP pinning,
  Certificate Transparency
- **Permissions** (`permissions.rs`) — 20 permission types
  (Geolocation, Camera, Microphone, Notifications, ...), per-(origin,
  permission) state, pluggable prompt handler, iframe allow parsing
- **Extensions** (`extensions.rs`) — Manifest V3, content scripts,
  match patterns (`<all_urls>`, `*://*.host/*`), glob matching,
  permissions, generate extension ID, ChromeApi enum
- **DevTools protocol** (`devtools.rs`) — JSON value type,
  Request/Response/Event/RpcError, Inspector/Page/Runtime/DOM/Network/
  Console methods, event subscribers, console message buffering

### Images (`image/`)
- **HTTP/HTTPS URLs** — fetched via `ureq`
- **`data:` URLs** — base64-encoded inline images
- **Local files** — relative paths resolved against the page URL
- **Formats** — PNG, JPEG, GIF (first frame), BMP (via the `image` crate)
- **Sizing** — `width` / `height` HTML attributes take precedence,
  CSS `width` / `height` respected, default 300×200px, nearest-neighbor
  scaling
- **Broken images** — grey placeholder box with the `alt` text
- **Caching** — global cache by URL

## What it does NOT do (yet)

- The `html::spec`, `dom::spec`, `css::spec`, `tjs_ext/`, `security/` modules are
  **structurally complete but not yet wired into the render pipeline**.
  Falco still uses the legacy `html` / `dom` / `css` modules for
  actual rendering. The new modules exist as the spec-compliant
  replacements and pass their own unit tests, but the renderer has
  not been switched over yet.
- No CSS animations / transitions in the renderer (the data structures
  exist in `css/spec/advanced.rs`, but the paint loop does not interpolate
  them).
- No `@media` query value matching in the renderer (queries are
  parsed; the cascade does not yet apply them conditionally).
- No real DOM mutation from JS in the renderer (`element.innerHTML =
  ...`, `element.style.color = ...` are stubs).
- No HTML5 spec-compliant tree repair in the renderer (the algorithm
  exists in `html/spec/tree_builder.rs` but the legacy parser is used).

## Architecture

| Module             | Lines  | Description                                                                |
|--------------------|--------|----------------------------------------------------------------------------|
| `html::spec`           | ~4,900 | WHATWG HTML5 tokenizer + tree builder + serializer + XML parser + encoding |
| `html.rs`          | ~540   | Legacy HTML parser (still used in render pipeline)                         |
| `dom::spec`            | ~2,280 | Spec-compliant DOM, MutationObserver, Shadow DOM, custom elements, a11y    |
| `dom.rs`           | ~140   | Legacy DOM (still used in render pipeline)                                 |
| `css::spec`            | ~2,020 | Selectors L4, cascade specificity, @-rules, animations, containment, filters |
| `css/`             | ~1,660 | Legacy CSS parser + selector matching + color parsing                      |
| `style/`           | ~1,720 | Style cascade + UA styles + inheritance + flex/grid properties             |
| `layout/`          | ~1,950 | Block / inline / flex / grid / table / float / absolute layout             |
| `paint/`           | ~470   | Canvas + font rasterizer + alpha compositing + gradients + shadows         |
| `svg/`             | ~1,130 | SVG parser + renderer (paths, shapes, gradients)                           |
| `tjs/`             | ~4,340 | Custom JS VM: lexer, parser, interpreter, bytecode VM, JIT, value, builtins |
| `tjs_ext/`         | ~780   | Symbol, BigInt, Promise, microtasks, Map/Set, WeakMap/WeakSet, Reflect     |
| `js_tjs.rs`        | ~700   | JS-to-DOM bindings (document, console, alert, onclick)                     |
| `web_runtime/`     | ~4,300 | fetch, XHR, event loop, Promise, WebGL, video, MSE, EME, NDSD, HTTP/2      |
| `net/`             | ~930   | HTTP fetch, cookies, cache, websocket, redirect                            |
| `security/`        | ~3,590 | SOP, multi-process, sandbox, CSP, certs, permissions, extensions, DevTools |
| `window/`          | ~1,110 | Interactive window: scrolling, forms, navigation, history, address bar    |
| `image/`           | ~200   | Image loader (HTTP, data: URLs, local files) + cache + scaling             |
| `png/`             | ~100   | Hand-written PNG encoder (no flate2 dependency)                            |
| `main.rs`, `lib.rs`| ~550   | CLI parsing + library entry points                                         |
| **Total**          | **~36,000** |                                                                        |

## Project layout

```
falco/
├── .github/                  # CI, issue templates, contributing, security policy
│   ├── workflows/
│   │   ├── ci.yml            # fmt + clippy + build + test on 3 OSes × 2 toolchains
│   │   └── release.yml       # Build per-OS release binaries on tag push
│   ├── ISSUE_TEMPLATE/       # bug_report.md, feature_request.md, config.yml
│   ├── CONTRIBUTING.md
│   ├── CODE_OF_CONDUCT.md
│   ├── SECURITY.md
│   ├── PULL_REQUEST_TEMPLATE.md
│   ├── FUNDING.yml           # MonoBank donation link
│   └── dependabot.yml
├── docs/                     # Logo
│   └── falco.svg             # project logo (by Sanya)
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── html/spec/                # WHATWG HTML5 (tokenizer, tree builder, ...)
│   ├── html.rs               # legacy HTML parser
│   ├── dom/spec/                 # spec DOM (observer, shadow, custom elements, a11y)
│   ├── dom.rs                # legacy DOM
│   ├── css/spec/                 # selectors L4, cascade, @-rules
│   ├── css/                  # legacy CSS parser
│   ├── style/                # cascade + inheritance + UA styles
│   ├── layout/               # block/inline/flex/grid/table/float/absolute
│   ├── paint/                # canvas + fonts + compositing
│   ├── svg/                  # SVG parser + renderer
│   ├── tjs/                  # custom JS VM (lexer, parser, VM, JIT)
│   ├── tjs_ext/              # Symbol, BigInt, Promise, ...
│   ├── web_runtime/          # fetch, XHR, event loop, WebGL, video, MSE, EME
│   ├── net/                  # HTTP, cookies, cache, websocket, redirect
│   ├── security/             # SOP, sandbox, CSP, certs, permissions, DevTools
│   ├── window/               # interactive window mode
│   ├── image/                # image loader
│   └── png/                  # PNG encoder
├── Cargo.toml
├── Cargo.lock
├── LICENSE
└── README.md
```

## Roadmap

The next big pieces of work, roughly in priority order:

1. **Wire `html::spec` tokenizer + tree_builder into the render pipeline**
   — replace the legacy `html::parse()` with the WHATWG-compliant parser.
   This unlocks spec-compliant tree repair (adoption agency, foster
   parenting), proper `<template>` handling, and correct misnested-tag
   recovery. The DOM mutation bridge (v0.2.0) already uses
   `html::spec::serializer`, so this is the natural next step.
2. **Wire `css::spec` into the cascade** — get `:is` / `:where` / `:has`
   working in real rendering, plus cascade layers and container
   queries.
3. **Async `fetch()` with Promise integration** — currently `fetch()` is
   blocking. Wire it through the `web_runtime/event_loop.rs` so it
   returns a real `Promise<Response>` and doesn't block the renderer.
4. **CSS animations / transitions** — interpolate keyframes in the
   paint loop, run them through the event loop.
5. **Wire `security/` into the renderer** — SOP enforcement in DOM
   access, CSP in the script runner, certificate validation on HTTPS
   fetches, multi-process sandbox.
6. **`@media` query value matching** — apply queries conditionally
   based on viewport size.

## Changelog

### v0.2.0

**Major release: the spec-compliant DOM is now wired into the render
pipeline. JavaScript can mutate the DOM and the changes are reflected
in the rendered output.**

- **Added**: DOM mutation from JavaScript now triggers re-render. The
  render pipeline now:
  1. Parses HTML into a legacy DOM (for layout compatibility).
  2. Converts the legacy DOM to a spec DOM via the new
     `js_tjs::legacy_dom_to_spec_document()` bridge.
  3. Creates a `TjsJsContext` with the spec DOM attached.
  4. Executes inline `<script>` tags — `element.innerHTML = ...`,
     `element.style.color = ...`, `document.getElementById(...).remove()`
     all mutate the live spec DOM.
  5. Serializes the (possibly mutated) spec DOM back to HTML via
     `js_tjs::serialize_spec_document()`.
  6. Re-parses the mutated HTML and runs layout + paint as before.

  This unlocks dynamic pages: SPAs, React/Vue-style rendering, any
  site that uses `innerHTML` or `appendChild` to build content.

- **Added**: real `fetch()` in the JS bridge. Previously `fetch()` was
  a stub. Now it uses `ureq` to make a blocking HTTP request and
  returns a response object with `ok`, `status`, `text()`, and `json()`
  methods. This unlocks AJAX-style sites that load content dynamically.

- **Added**: 6 new `String.prototype` methods on the JS VM:
  `repeat`, `padStart`, `padEnd`, `trimStart`, `trimEnd`. Combined
  with the existing String methods, this brings Falco's JS String
  support close to the ES2015+ spec.

- **Added**: 8 new regression tests for String methods. Total: 341
  tests passing.

- **Changed**: bumped version to 0.2.0 (minor bump — new features,
  no breaking changes to public API).

- **Known limitations**:
  - The spec HTML5 parser (`html::spec::tokenizer` + `tree_builder`)
    is still not wired in — the legacy `html::parse()` is used for
    the initial parse. The spec parser is structurally complete and
    unit-tested but requires the legacy DOM to be replaced entirely.
    Planned for v0.3.0.
  - `fetch()` is blocking (no Promise, no async). The event loop in
    `web_runtime/event_loop.rs` is real and unit-tested, but the JS
    bridge doesn't yet integrate with it. Planned for v0.3.0.
  - The security module (SOP, CSP, sandbox, cert validation) is still
    not enforced in the renderer.

### v0.1.1

- **Added**: `render_to_buffer()` public API returning raw RGBA pixels
  (no PNG encoding), for embedding Falco into game engines and GUI
  toolkits. The new `RenderedBuffer` struct exposes `as_rgba()`,
  `to_bgra()` (for DirectX/Vulkan/Win32), and `to_rgb()`.
- **Fixed**: whitespace collapsed in rendered text. Spaces between
  words had zero width because outline-less glyphs returned `None`,
  so the paint loop didn't advance the caret. Now returns a transparent
  glyph with the font's real horizontal advance. (thanks @d0sch1, PR #6)
- **Added**: 14 real `Array.prototype` methods on the JS VM
  (`map`, `filter`, `reduce`, `forEach`, `find`, `some`, `every`,
  `slice`, `concat`, `includes`, `indexOf`, `reverse`, `push`, `pop`,
  `join`). Previously they were stubs or missing entirely. Callback
  methods invoke user functions via a new `call_js` helper in
  `tjs/interpreter.rs`. (thanks @d0sch1, PR #6)
- **Added**: 10 new regression tests for Array methods and closure
  capture in `tjs/mod.rs`. Total: 333 tests passing.
- **Added**: Benchmarks section to README.
- **Added**: Programmatic API section with `render_to_buffer` example.

### v0.1.0

- Initial public release.
- HTML/CSS/layout/paint pipeline working end-to-end.
- Interactive `--window` mode with scrolling, forms, navigation.
- Spec-compliant `html::spec/`, `dom::spec/`, `css::spec/` modules
  structurally complete but not yet wired into the render pipeline.
- Security module (SOP, CSP, sandbox, certs, permissions) implemented
  but not enforced in the renderer.

## Support the project

If Falco is useful to you, consider buying the author a coffee:

<p align="center">
  <a href="https://send.monobank.ua/jar/21T9ZRwZq3">
    <img alt="MonoBank donation" src="https://img.shields.io/badge/Support%20on-MonoBank-7c3aed.svg?style=for-the-badge&logo=monobank&logoColor=white">
  </a>
</p>

## License

MIT — see [LICENSE](LICENSE).
