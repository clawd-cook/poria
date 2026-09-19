# CI/CD workflow specifications

These files describe **what** the GitHub Actions in `.github/workflows/` must do. The YAML is the implementation. When they disagree, fix the YAML or update the matching spec in the same change.

| Workflow or action            | Spec                                                                               | Trigger                                        |
| ----------------------------- | ---------------------------------------------------------------------------------- | ---------------------------------------------- |
| Pre-Publish (Beta)            | [spec-process-cicd-pre-publish.md](./spec-process-cicd-pre-publish.md)             | Tag `vX.Y.Z-beta.N`                            |
| Publish (Stable)              | [spec-process-cicd-publish.md](./spec-process-cicd-publish.md)                     | Tag `vX.Y.Z` (no prerelease suffix)            |
| Warm Rust cache               | [spec-process-cicd-warm-rust-cache.md](./spec-process-cicd-warm-rust-cache.md)     | Push to `main` on Rust paths, or manual        |
| Labeler                       | [spec-process-cicd-labeler.md](./spec-process-cicd-labeler.md)                     | PR open/sync; `labels.yml` push to `main`      |
| Setup macOS Tauri (composite) | [spec-process-cicd-setup-macos-tauri.md](./spec-process-cicd-setup-macos-tauri.md) | Called by publish, pre-publish, and cache warm |

Product install and tag recipes live in [README.md](../README.md). Crate layout lives in [ARCHITECTURE.md](../ARCHITECTURE.md).
