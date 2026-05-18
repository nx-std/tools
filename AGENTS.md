# nx-std/tools - Technical Overview for Coding Agents

## Project Summary

`nx-std/tools` is a Rust workspace containing **host-targeted** development tools for Nintendo Switch homebrew:

- **cargo-nx** — Cargo subcommand for creating and building Switch homebrew projects (NRO/NSP)
- **nx-netloader** — Library for network deployment of homebrew to a Switch console

These are **host tools**: they compile and run on the development machine. No cross-compilation, no Meson, no devkitPro needed at build time.

## Quick Start

**If you're an AI agent working on this codebase, here's what you need to know immediately:**

1. **Invoke `/code-guidelines` FIRST** → Before planning OR coding (applies in Plan mode too), load the relevant guidelines for the affected crate(s). Skipping this leads to plans that violate project conventions and cause rework.
2. **Use Skills for operations** → Invoke skills (`/code-format`, `/code-check`, `/code-test`, `/code-review`) instead of running commands directly.
3. **Skills wrap justfile tasks** → Skills provide the interface to `just` commands with proper guidance.
4. **Follow the workflow** → Format → Check → Clippy → Test.
5. **Fix ALL warnings** → Zero tolerance for clippy warnings.

**Your first action**: Invoke `/code-guidelines` to load guidelines before drafting a plan or writing any code. For commands, invoke the relevant Skill.

## Table of Contents

1. [Principles](#1-principles) — Core design principles guiding this codebase
2. [Code Guidelines](#2-code-guidelines) — Understanding coding standards via `/code-guidelines` skill
3. [Architecture](#3-architecture) — Crates and their relationships
4. [Build System](#4-build-system) — Cargo workspace, prerequisites
5. [Development Workflow](#5-development-workflow) — How to develop with this codebase


## 1. Principles

**MANDATORY**: Before writing any code, read and internalize these core design principles. Full details, examples, and checklists are in the linked docs — read them every time.

| Principle                   | One-liner                                                        | Full doc                                              |
|-----------------------------|------------------------------------------------------------------|-------------------------------------------------------|
| **Single Responsibility**   | One struct = one reason to change                                | @docs/code/principle-single-responsibility.md         |
| **Open/Closed**             | Extend via new types/trait impls, don't modify existing code     | @docs/code/principle-open-closed.md                   |
| **Law of Demeter**          | Only talk to immediate collaborators — no `a.b().c().d()` chains | @docs/code/principle-law-of-demeter.md                |
| **Validate at the Edge**    | Hard shell (boundary validates), soft core (domain trusts)       | @docs/code/principle-validate-at-edge.md              |
| **Type-Driven Design**      | Make illegal states unrepresentable via the type system          | @docs/code/principle-type-driven-design.md            |
| **Idempotency**             | Operations safe to retry — same effect whether run once or N×    | @docs/code/principle-idempotency.md                   |
| **Inversion of Control**    | Depend on abstractions, not concretions                          | @docs/code/principle-inversion-of-control.md          |
| **Least Surprise**          | Code behaves the way readers expect from its name and signature  | @docs/code/principle-least-surprise.md                |
| **DRY/WET balance**         | Deduplicate real knowledge; tolerate incidental similarity       | @docs/code/principle-dry-wet.md                       |

Use `/code-guidelines principles` to load these on demand when relevant to your task.


## 2. Code Guidelines

Code guideline documentation lives in `docs/code/` with YAML frontmatter for dynamic discovery.

**Guideline docs are authoritative**: Guideline docs define how code should be written. All implementations MUST follow the patterns. If code doesn't follow a pattern, either fix the code or update the pattern.

### Guideline Types

| Type               | Scope          | Purpose                                                          |
|--------------------|----------------|------------------------------------------------------------------|
| **Principle**      | `global`       | Universal software principles and best practices                 |
| **Core**           | `global`       | Fundamental coding standards (error handling, logging, modules)  |
| **Architectural**  | `global`       | High-level patterns (workspace structure, crate layout)          |
| **Pattern**        | `global`       | Reusable design patterns (builder, typestate)                    |
| **Crate-specific** | `crate:<name>` | Patterns for specific crates (`cargo-nx`, `nx-netloader`)        |
| **Meta**           | `global`       | Documentation format specifications (`docs/__meta__/`)           |

### Skill Invocation

| When You Need To                                                  | Invoke This Skill   |
|-------------------------------------------------------------------|---------------------|
| Understand code guidelines before implementing                    | `/code-guidelines`  |
| "How should I handle errors?", "What's the pattern for X?"        | `/code-guidelines`  |
| Load crate-specific guidelines for `cargo-nx` or `nx-netloader`   | `/code-guidelines`  |
| Review code changes for guideline compliance                      | `/code-review`      |

**Navigation:**

- Need to understand patterns? → `/code-guidelines`
- All guidelines located in `docs/code/`
- Documentation format specs in `docs/__meta__/`

### Code Style

- Uses unstable `rustfmt` features (nightly required for formatting)
- Stable toolchain (`1.93.0`) for compilation, pinned via `rust-toolchain.toml`
- Imports grouped: std, external crates, local (`group_imports = "StdExternalCrate"`)
- Import granularity at crate level (`imports_granularity = "Crate"`)


## 3. Architecture

`cargo-nx` depends on `nx-netloader`; `nx-netloader` is otherwise standalone.

### What These Tools Do

- **cargo-nx**: drives `cargo build` against the Switch target, packages outputs as NRO/NSP for upload, and invokes `nx-netloader` for deployment.
- **nx-netloader**: implements the host side of the nxlink protocol used by Atmosphère's `nxlink` daemon to receive NROs over the network.

Neither crate cross-compiles to the Switch — they run on the developer's machine and orchestrate the build/deploy of *other* projects that do.


## 4. Build System

The project uses a **standard Cargo workspace** driven by `just` recipes.

### Prerequisites

- Rust stable toolchain `1.93.0` (specified in `rust-toolchain.toml`) for compilation
- Rust nightly toolchain for formatting (required by unstable rustfmt features)
- `just` command runner
- Optional: `cargo-nextest` for faster test runs (`just test` falls back to `cargo test`)

### Build

```
cargo build              # debug build
cargo build --release    # release build
```

Standard cargo — there is no extra build orchestration.


## 5. Development Workflow

**This section provides guidance for AI agents on how to develop with this codebase.**

### Documentation Structure: Separation of Concerns

This project uses three complementary documentation systems. Understanding their roles helps AI agents navigate efficiently:

| Documentation                  | Purpose                  | Content Focus                                                                                                                                |
|--------------------------------|--------------------------|----------------------------------------------------------------------------------------------------------------------------------------------|
| **AGENTS.md** (this file)      | **WHY** and **WHAT**     | Project architecture, policies, goals, and principles. Answers "What is this project?" and "Why do we do things this way?"                   |
| **Skills** (`.agents/skills/`) | **HOW** and **WHEN**     | Command-line operations and just usage. Answers "How do I run commands?" and "When should I use each command?"                               |
| **Guidelines** (`docs/code/`)  | **HOW** (implementation) | Code implementation guidelines and standards (see [Code Guidelines](#2-code-guidelines)). Answers "How do I write quality, conventional code?"|

**Navigation Guide for AI Agents:**

- Need to understand the project? → Read this file (AGENTS.md)
- Need to run a command? → Invoke the appropriate Skill (`/code-format`, `/code-check`, `/code-test`)
- Need to write code? → Use `/code-guidelines` to load relevant guidelines

### Core Operating Principle

**🚨 MANDATORY: USE Skills for all common operations. Skills wrap just tasks with proper guidance.**

#### The Golden Rule

**USE Skills (`/code-format`, `/code-check`, `/code-test`, `/code-review`) for all common operations. Only use `cargo` or `just` directly when the operation is NOT covered by a skill.**

**Decision process:**

1. **First**: Check if a skill exists for your operation
2. **If exists**: Invoke the skill (provides proper flags, setup, and error handling)
3. **If not exists**: You may run the tool directly (e.g., one-off `cargo` introspection commands)

#### Why Skills Are Preferred

- **Consistency**: Uniform command execution across all developers and AI agents
- **Correctness**: Skills ensure proper flags and toolchain selection
- **Guidance**: Skills provide context on when and how to use commands
- **Pre-approved workflows**: Skills document which commands can run without user permission

#### Examples

- ✅ **Use skill**: `/code-format` (formatting Rust)
- ✅ **Use skill**: `/code-check` (compile check, clippy)
- ✅ **Use skill**: `/code-test` (run cargo tests)
- ✅ **Use skill**: `/code-review` (review changes against guidelines)
- ✅ **Direct tool OK**: `cargo tree -p cargo-nx` (introspection not in justfile)

#### Command Execution Hierarchy (Priority Order)

When determining which command to run, follow this strict hierarchy:

1. **Priority 1: Skills** (`.agents/skills/`)
   - Skills are the **SINGLE SOURCE OF TRUTH** for all command execution
   - If a Skill documents a command, use it EXACTLY as shown
   - Skills override any other guidance in AGENTS.md or elsewhere

2. **Priority 2: AGENTS.md workflow**
   - High-level workflow guidance (when to format, check, test)
   - Refers you to Skills for specific commands

3. **Priority 3: Everything else**
   - Other documentation is supplementary
   - When in conflict, Skills always win

#### Workflow Gate: Use Skills First

**Before running ANY command:**

1. Ask yourself: "Which Skill covers this operation?"
2. Invoke the appropriate skill (e.g., `/code-format`, `/code-check`, `/code-test`)
3. Let the skill guide you through the operation

**Example decision tree:**

- Need to format a file? → Use `/code-format` skill
- Need to check a crate? → Use `/code-check` skill
- Need to run tests? → Use `/code-test` skill
- Need to review changes? → Use `/code-review` skill

### Command-Line Operations Reference

**🚨 CRITICAL: Use skills for all operations — invoke them before running commands.**

Available skills and their purposes:

- **Formatting**: `/code-format` — Format Rust code after editing files
- **Checking/Linting**: `/code-check` — Validate compilation and lint with clippy
- **Testing**: `/code-test` — Run cargo tests
- **Guidelines**: `/code-guidelines` — Load relevant code guidelines and patterns
- **Reviewing**: `/code-review` — Review changes for bugs, guideline violations, security

Each Skill provides:

- ✅ **When to use** — Clear guidance on appropriate usage
- ✅ **Available operations** — All supported tasks with proper execution
- ✅ **Examples** — Real-world usage patterns
- ✅ **Pre-approved workflows** — Operations that can run without user permission
- ✅ **Workflow integration** — How operations fit into the development flow

**Remember: If you don't know which operation to perform, invoke the appropriate Skill.**

### Pre-Implementation Checklist

**BEFORE drafting a plan OR writing ANY code, you MUST:**

1. **Understand the task** — Research the codebase and identify affected crate(s)
2. **🚨 MANDATORY: Load implementation guidelines FIRST** — Invoke `/code-guidelines` before drafting any plan or writing any code. This applies equally in Plan mode: the plan itself MUST be grounded in the loaded guidelines, not in assumptions about conventions.
3. **Follow crate-specific guidelines** — Guideline discovery loads crate-specific and core guidelines automatically
4. **Rationale** — Skipping this step leads to plans that violate conventions, causing avoidable rework.

### Typical Development Workflow

**Follow this workflow when implementing features or fixing bugs:**

#### 1. Research Phase

- Understand the codebase and existing guidelines
- Identify related modules and dependencies
- Use `/code-guidelines` to load relevant implementation guidelines

#### 2. Planning Phase

**🚨 MANDATORY FIRST STEP (including in Plan mode):** Invoke `/code-guidelines` to load the guidelines for the affected crate(s) BEFORE drafting the plan. The plan's structure, module layout, error handling, and type design decisions MUST reflect the loaded guidelines.

- Create the implementation plan on top of the loaded guidelines
- Ensure plan follows required patterns (error handling, type design, module structure)
- Identify validation checkpoints
- Consider edge cases and error handling according to guidelines
- Ask user questions if requirements are unclear

#### 3. Implementation Phase

**🚨 CRITICAL: Before running ANY command in this phase, invoke the relevant Skill.**

**Copy this checklist and track your progress:**

```
Development Progress:
- [ ] Step 1: Write code following guidelines (use /code-guidelines)
- [ ] Step 2: Format code (use /code-format skill)
- [ ] Step 3: Check compilation (use /code-check skill)
- [ ] Step 4: Fix all compilation errors
- [ ] Step 5: Run clippy (use /code-check skill)
- [ ] Step 6: Fix ALL clippy warnings
- [ ] Step 7: Run tests when warranted (use /code-test skill)
- [ ] Step 8: All required checks pass ✅
```

**Detailed workflow for each work chunk (and before committing):**

1. **Write code** following guidelines from [Code Guidelines](#2-code-guidelines) (loaded via `/code-guidelines`)

2. **Format before checks/commit**:
   - **Use**: `/code-format` skill when you finish a coherent chunk of work
   - **Validation**: Verify no formatting changes remain

3. **Check compilation**:
   - **Use**: `/code-check` skill after changes
   - **Must pass**: Fix all compilation errors
   - **Validation**: Ensure zero errors before proceeding

4. **Lint with clippy**:
   - **Use**: `/code-check` skill for linting
   - **Must pass**: Fix all clippy warnings
   - **Validation**: Re-run until zero warnings before proceeding

5. **Run tests (when warranted)**:
   - **Prerequisite**: `/code-format` and `/code-check` must both be clean
   - **Use**: `/code-test` skill — runs `cargo test` (or `cargo nextest run`) at per-crate or workspace scope
   - **When to run**: Behavior changes, public API changes, or new logic in tested code paths
   - **Validation**: Fix failures or record why tests were skipped

6. **Iterate**: If any validation fails → fix → return to step 2

**Visual Workflow:**

```
Edit File → /code-format skill
          ↓
    /code-check skill (compile) → Fix errors?
          ↓                            ↓ Yes
    /code-check skill (clippy) → (loop back)
          ↓
    ALL CHECKS GREEN ─ gate
          ↓
    /code-test skill (when warranted)
          ↓              ↓ Fix failure?
    All Pass ✅     (loop back to /code-format)
```

**Remember**: Invoke Skills for all operations. If unsure which skill to use, refer to the Command-Line Operations Reference above.

#### 4. Completion Phase

- Ensure all required checks pass (format, check, clippy, tests)
- If tests were skipped, document why and the risk assessment
- Review changes against guidelines (use `/code-review`)
- Document any warnings you couldn't fix and why

### Core Development Principles

**ALL AI agents MUST follow these principles:**

- **Consistency and homogeneity are fundamental**: The codebase must read as if written by a single author. All new code
  must match the style, structure, and conventions of the surrounding code. Deviating from an established pattern requires
  strong, explicit justification — "I prefer it this way" is not sufficient. If you believe a pattern should change, propose
  the change to the pattern documentation first; do not introduce one-off divergences.
- **Research → Plan → Implement**: Never jump straight to coding
- **Guidelines before planning**: `/code-guidelines` is a prerequisite for both planning AND Plan mode, not just implementation. A plan written without loaded guidelines is considered incomplete.
- **Guideline compliance**: Follow guidelines from [Code Guidelines](#2-code-guidelines)
- **Zero tolerance for errors**: All automated checks must pass
- **Clarity over cleverness**: Choose clear, maintainable solutions

**Essential conventions:**

- **Maintain type safety**: Leverage Rust's type system fully (see [Type-Driven Design](docs/code/principle-type-driven-design.md))
- **Validate at boundaries**: Hard shell on user-facing CLI/network surfaces; soft core inside (see [Validate at the Edge](docs/code/principle-validate-at-edge.md))
- **Format code before checks/commit**: Use `/code-format` skill
- **Fix all warnings**: Use `/code-check` skill for clippy
- **Test behavior changes**: Use `/code-test` skill after checks are green

### Summary: Key Takeaways for AI Agents

| What                | Where                                  | When                                                  |
|---------------------|----------------------------------------|-------------------------------------------------------|
| **Plan work**       | `/code-guidelines`                     | BEFORE creating any plan                              |
| **Run commands**    | `.agents/skills/`                      | Check Skills BEFORE any command                       |
| **Write code**      | [Code Guidelines](#2-code-guidelines)  | Load guidelines before implementation                 |
| **Format**          | `/code-format`                         | Before checks or before committing                    |
| **Check**           | `/code-check`                          | After formatting                                      |
| **Lint**            | `/code-check`                          | Fix ALL warnings                                      |
| **Test**            | `/code-test`                           | After checks green; on behavior changes               |
| **Review**          | `/code-review`                         | Before commits / PRs                                  |

**Golden Rules:**

1. ✅ Invoke Skills for all common operations
2. ✅ Skills wrap just tasks with proper guidance
3. ✅ Follow the workflow: Format → Check → Clippy → Test
4. ✅ Zero tolerance for errors and warnings
5. ✅ Every change improves the codebase

**Remember**: When in doubt, invoke the appropriate Skill!
