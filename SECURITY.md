# OSMIIC Security & Audit Policy

## Pre-Release Security Checklist

Before cutting any production release tag for OSMIIC, perform the following verification steps:

1. **Dependency Vulnerability Scan (`cargo audit`)**:
   ```bash
   cd src-tauri
   cargo audit
   ```
   Confirm that zero unmitigated security advisories (RUSTSEC) affect OSMIIC or its direct/transitive dependencies.

2. **Linter & Static Analysis (`cargo clippy`)**:
   ```bash
   cd src-tauri
   cargo clippy --all-targets -- -W clippy::all
   ```
   Ensure zero clippy warnings exist.

3. **Backend Unit Tests**:
   ```bash
   cd src-tauri
   cargo test
   ```
   Ensure all unit tests pass (language classifier, filename sanitization, disk duplicate detection).

4. **Webview Content Security Policy (CSP)**:
   Verify `tauri.conf.json` enforces a strict CSP object and does not set `"csp": null` or allow `'unsafe-inline'` script execution.
