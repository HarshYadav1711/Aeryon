# Contributing to Aeryon

Thank you for your interest in contributing. This document describes the process and expectations for contributions.

## Before You Start

1. Read the [README](README.md) to understand project scope and architecture.
2. Check [ROADMAP.md](ROADMAP.md) and open issues to avoid duplicating planned work.
3. For significant design changes, open an issue or draft an [Architecture Decision Record](docs/adr/) before implementing.

## Development Setup

Setup instructions will be added as build tooling is introduced. Until then, follow the directory conventions in the README when adding new modules.

## Contribution Workflow

1. Fork the repository and create a feature branch from `main`.
2. Make focused changes with clear commit messages.
3. Ensure tests pass (once the test harness is in place).
4. Open a pull request with a description of what changed and why.

### Commit Messages

Use imperative mood and keep the subject line under 72 characters:

```
Add plugin interface trait for sensor backends
Fix calibration metadata serialization
```

Reference issue numbers in the body when applicable.

## Code Standards

- **Rust:** Follow standard `rustfmt` and `clippy` conventions.
- **C++:** Follow the style established in `native/cpp-dsp/` (to be defined).
- **Python:** Follow PEP 8. Use type hints for public APIs.
- **TypeScript:** Follow the ESLint configuration in `frontend/` (to be added).

## Architecture Decisions

Non-trivial design choices should be documented as ADRs in `docs/adr/`. Use the naming convention `NNNN-short-title.md` (e.g., `0001-plugin-interface.md`).

## Pull Request Guidelines

- Keep PRs small and reviewable. Prefer a series of focused PRs over one large change.
- Include tests for new behavior where applicable.
- Update documentation when changing public interfaces.
- Do not commit secrets, credentials, or large binary datasets.

## Reporting Issues

Use GitHub Issues for bug reports and feature requests. Include:

- Steps to reproduce (for bugs)
- Expected vs. actual behavior
- Environment details (OS, compiler/runtime versions)

## Code of Conduct

All contributors are expected to follow the [Code of Conduct](CODE_OF_CONDUCT.md).
