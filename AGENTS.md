# Tools Workspace - Agent Guide

## Project Summary

This is a Rust workspace containing host-targeted development tools for Nintendo Switch homebrew:

- **cargo-nx** - Cargo subcommand for creating and building Switch homebrew projects (NRO/NSP)
- **netloader** - Library for network deployment of homebrew to a Switch console

These are **host tools** - they compile and run on the development machine (no cross-compilation, no Meson, no devkitPro
needed at build time).

## Quick Start

**If you're an AI agent working on this codebase, here's what you need to know immediately:**

1. **Use Skills for operations** - Invoke skills (`/code-fmt`, `/code-check`, `/code-test`) instead of running commands
   directly
2. **Skills wrap justfile tasks** - Skills provide the interface to `just` commands with proper guidance
3. **Follow the workflow** - Format -> Check -> Clippy -> Tests (when needed)
4. **Fix ALL warnings** - Zero tolerance for clippy warnings

**Your first action**: If you need to run a command, invoke the relevant Skill.

## Skills Reference

| Operation         | Skill         | When to Use                                         |
|-------------------|---------------|-----------------------------------------------------|
| Format code       | `/code-fmt`   | After editing .rs files, before checks/commits      |
| Check compilation | `/code-check` | After formatting, to validate code compiles         |
| Lint with clippy  | `/code-check` | After compilation check passes                      |
| Run tests         | `/code-test`  | After checks pass, when behavior changes warrant it |

Each Skill provides command selection rules, available commands, examples, and anti-patterns.

## Development Workflow

### Workflow for Every Code Change

```
Edit File -> /code-fmt skill
          |
    /code-check skill (compile) -> Fix errors?
          |                            | Yes
    /code-check skill (clippy) -> (loop back)
          |
    Tests if needed (/code-test) -> Fix failures?
          |                    | Yes
    All Pass               (loop back)
```

### Implementation Checklist

```
- [ ] Write code
- [ ] Format code (use /code-fmt skill)
- [ ] Check compilation (use /code-check skill)
- [ ] Run clippy (use /code-check skill)
- [ ] Fix ALL warnings
- [ ] Run tests when warranted (use /code-test skill)
- [ ] All checks pass
```

### Core Principles

- **Use Skills first**: Check if a skill exists before running any command directly
- **Format before checks**: Always format before running compilation or lint checks
- **Zero tolerance for warnings**: All clippy warnings must be fixed
- **Smallest test scope**: Run only the tests relevant to your change

## Code Style

- Uses unstable `rustfmt` features (requires nightly for formatting)
- Stable toolchain (`1.93.0`) for compilation
- Imports: grouped as std, external crates, local (`group_imports = "StdExternalCrate"`)
- Import granularity: crate level (`imports_granularity = "Crate"`)
