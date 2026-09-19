# Security Policy

Poria is a local Tauri desktop app. It stores SSO cookies, clones git repos, and runs Claude CLI against worktrees. Treat credential files, IPC, agent tool access, and signing secrets as sensitive.

请勿在公开 Issue、Discussion 或 Pull Request 中报告安全漏洞。请使用下方的私下披露渠道。

## Supported versions

Security fixes ship on the latest stable `vX.Y.Z` GitHub Release (currently **1.1.x**, latest tag `v1.1.1`).

The 2.0 line is **pre-release only** (`v2.0.x-beta.*`, current `v2.0.1-beta.1`). Pre-releases are not a supported security surface.

| Version                                                   | Supported                                             |
| --------------------------------------------------------- | ----------------------------------------------------- |
| 1.1.x                                                     | Yes                                                   |
| 1.0.x                                                     | Best effort for high-severity issues that still apply |
| `v2.0.x-beta*`, other `v*-beta*` / `v*-rc*` / `v*-alpha*` | No                                                    |
| &lt; 1.0                                                  | No                                                    |

Unsigned or ad-hoc signed DMGs from CI are expected when Apple notarization secrets are not configured. That is a packaging limitation, not a supported hardened release. Current publish workflows build **Apple Silicon** (`aarch64-apple-darwin`) only.

## Reporting a vulnerability

**Do not** file a public GitHub issue, discussion, or PR for security reports.

Preferred:

1. [GitHub private vulnerability report](https://github.com/clawd-cook/poria/security/advisories/new) for this repository.

If that form is unavailable, email **mail@heyq02.cn** with subject `Poria security: <short title>`.

Include as much of the following as you can:

- Affected version, tag, or commit SHA
- What is impacted (credentials, worktree escape, IPC, signing, CI secrets, agent tool abuse, opener/shell permissions)
- Step-by-step reproduction or a proof of concept
- Relevant paths (`src-tauri/…`, `crates/…`) and logs **with cookies and tokens redacted**
- Suggested fix, if you have one

You should receive an acknowledgement within **3 business days**. If the report is confirmed, we will work on a fix and coordinate public disclosure (usually via a GitHub Security Advisory). If it is declined, we will say why.

We do not pay a bug bounty.

## Scope

In scope:

- Theft or leakage of SSO cookies / Coding credentials (`~/.poria/auth.json`, login callback, IPC)
- Path traversal that reads, writes, or deletes outside `~/.poria/repos`, `~/.poria/projects`, or `~/.poria/workspaces`
- Agent or skill behavior that bypasses output-guard / worktree limits and mutates unexpected paths (hosted clones, projects docs, paths outside the pipeline workspace)
- Privilege issues in Tauri commands (`src-tauri/src/commands/`) that expose secrets to the webview or the network
- Abuse of shell / opener capabilities (including opening or executing paths outside `$HOME/.poria/workspaces`)
- Supply-chain issues in the published GitHub Release DMG or in release workflows (secret exfiltration, malicious signing)

Out of scope:

- Bugs that require an attacker who already has the same macOS user account (they can already read `~/.poria`)
- Vulnerabilities in Xingyun, JoySpace, Coding, EasyCI, or Claude itself — report those upstream
- Issues only inside `submodules/` — report the submodule’s own security policy
- Missing Apple Developer notarization when `APPLE_*` GitHub secrets are empty
- Social engineering of SSO login
- Vite-only `http://localhost:1420` (no Tauri IPC); report desktop-window issues instead

## Handling secrets and local data

- Auth is stored at `~/.poria/auth.json` with mode `0600`. Do not copy this file into the repo, issues, or CI logs.
- App config (`claude_path` and gate thresholds) is `~/.poria/config.json`. Do not commit it.
- Hosted clones, demand markdown, and pipeline workspaces live under `~/.poria/`. Do not commit that tree.
- SQLite may contain pipeline metadata under `~/Library/Application Support/com.poria.desktop/` (or `workspace/db/poria.db` in some local runs).
- Opener is limited to `$HOME/.poria/workspaces` and children (`src-tauri/capabilities/default.json`). Do not widen shell/opener permissions without review.
- Cookie values in logs should go through `redact_cookie` (or equivalent). Do not paste raw `Cookie` headers.
- Release workflows may use `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`, and `TAURI_SIGNING_PRIVATE_KEY`. Keep them as GitHub Actions secrets; never commit them.

## Coordinated disclosure

Please give us a reasonable window to patch a confirmed issue before public write-ups. We will credit reporters in the advisory if they want to be named.
