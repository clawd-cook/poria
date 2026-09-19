---
title: CI/CD workflow specification: Setup macOS Tauri
owner: clawd-cook
tags: [process, cicd, github-actions, tauri, rust, composite]
source: .github/actions/setup-macos-tauri/action.yml
---

## Workflow overview

**Purpose**: Shared macOS job setup: stable Rust with a target triple, Cargo registry and `target/` cache, optional prebuilt `cargo-tauri`.
**Trigger**: Composite action, not a workflow. Callers are pre-publish, publish, and warm-rust-cache.
**Target**: GitHub `macos-latest` runners.

## Inputs

| Name          | Required | Default | Meaning                                                        |
| ------------- | -------- | ------- | -------------------------------------------------------------- |
| `target`      | yes      |         | rustc triple, currently `aarch64-apple-darwin` in all callers  |
| `save-cache`  | no       | `false` | Persist the Cargo cache. `true` only on the main-branch warmer |
| `install-cli` | no       | `true`  | Install `cargo-tauri`. `false` on the warmer                   |

## Requirements

| ID      | Requirement                                                                    | Priority | Acceptance                                             |
| ------- | ------------------------------------------------------------------------------ | -------- | ------------------------------------------------------ |
| REQ-001 | Stable Rust plus the requested target                                          | High     | `rustc` can compile for `target`                       |
| REQ-002 | Cache key includes the target and a `release` suffix                           | High     | Warmer save matches publish restore                    |
| REQ-003 | Cache save only when `save-cache` is `true`                                    | High     | Tag builds restore but do not overwrite the main cache |
| REQ-004 | Cache still writes on compile failure when saving                              | Medium   | `cache-on-failure: true`                               |
| REQ-005 | When installing CLI, use the pinned prebuilt `cargo-tauri` zip for that target | High     | Not `cargo install tauri-cli` on the runner            |
| REQ-006 | CLI pin is `2.11.4`                                                            | High     | Cache key and download URL use that version            |
| REQ-007 | `PATH` contains `~/.cargo/bin` after CLI install                               | High     | `cargo tauri --version` succeeds                       |

## Error handling

| Error                     | Response                     | Recovery                                              |
| ------------------------- | ---------------------------- | ----------------------------------------------------- |
| Zip missing `cargo-tauri` | Fail the step                | Check the GitHub Releases asset name for that version |
| Cache miss                | Download CLI or cold-compile | Expected on a new pin or first run                    |

## Related

- Implementation: `.github/actions/setup-macos-tauri/action.yml`
- Callers: [Pre-Publish](./spec-process-cicd-pre-publish.md), [Publish](./spec-process-cicd-publish.md), [Warm Rust cache](./spec-process-cicd-warm-rust-cache.md)
