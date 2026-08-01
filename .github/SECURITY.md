# Security Policy

## Supported versions

Falco is a hobby / research browser engine. Security fixes are applied
only to the latest release line.

| Version | Supported          |
| ------- | ------------------ |
| latest  | :white_check_mark: |
| < latest | :x:               |

## Reporting a vulnerability

**Please do not file public issues for security vulnerabilities.**

Instead, please report them privately:

1. Go to the **Security** tab of this repo.
2. Click **Report a vulnerability**.
3. Describe the issue, include a reproducer if possible.

We will acknowledge within 72 hours and aim to ship a fix within 30
days for high-severity issues.

## Threat model (and what Falco is NOT)

Falco is **not a production browser**. It is a small engine intended
for learning, headless rendering, and embedding in tooling. In
particular:

- Falco does **not** yet enforce a complete same-origin policy in its
  render pipeline. The `security/` module implements the spec
  algorithms but they are not fully wired into the renderer.
- Falco does **not** yet run JS in a sandboxed process.
- Falco's HTML/CSS/JS parsers are **not** hardened against malicious
  input that attempts to exploit parser bugs to escape the engine.

**Do not** use Falco to render untrusted content in a security-sensitive
context. Treat Falco the way you would treat a research prototype.

## Hardening roadmap

Items we know we need (in priority order):

1. Wire `security::origin` SOP checks into the DOM and fetch pipeline.
2. Run the JS engine in a child process with `security::sandbox`
   seccomp filters applied.
3. Site-isolation: one renderer process per origin (see
   `security::process`).
4. CSP enforcement in the HTML parser / script runner (see
   `security::csp::allows_inline_script`).
5. TLS certificate validation by default on HTTPS fetches (see
   `security::cert::validate_chain`).
