---
name: code-guidelines
description: Load relevant code guidelines based on user query or work context. Use when asking about code guidelines, standards, principles, or before implementing code in the nx-std/tools workspace.
allowed-tools: Bash(grep *)
---

# Code Guideline Discovery Skill

Lazy-loads `nx-std/tools` code guideline docs from `docs/code/` based on user query or work context, via YAML frontmatter matching.

## When to Use

- User asks about guidelines, standards, principles, or "how should I handle X?"
- Before implementing code, to load the relevant crate and core guidelines
- User asks about error handling, logging, testing, or documentation patterns

## Prefetched Guideline Catalog

The frontmatter of all guideline docs (loaded at skill-start):

!`grep -m 4 -E '^(name|description|type|scope):' docs/code/*.md`

> **Fallback**: if the block above appears as literal text (runtime does not auto-execute dynamic context), run it yourself with the Bash tool before proceeding.

## Workflow

1. **Match** the user's query or work context against the prefetched catalog using the fields below.
2. **Load** matched docs with the Read tool: `docs/code/<name>.md`.

## Query Matching

Compare query/context against frontmatter fields:

- `name` — exact or partial match (e.g. "errors-handling", "rust-modules")
- `description` — semantic match, especially "Load when …" triggers
- `type` — `core`, `arch`, `crate`, `meta`, `principle`
- `scope` — `global` or `crate:<name>`

### Priority when multiple match

1. Crate-specific (`scope: crate:<name>`) — most specific
2. Core (`type: core`) — fundamental standards
3. Architectural (`type: arch`) — high-level organization
4. Meta (`type: meta`) — only when creating/editing docs

### Common semantic matches

- "how do I log errors?" → `logging`, `logging-errors`
- "how to document functions?" → `rust-documentation`
- defining error types → `errors-handling`, `errors-reporting`
- writing tests → `test-functions`, `test-files`, `test-organization`
- creating crates → `rust-workspace`, `rust-crate`
- crate-specific work (e.g. `cargo-nx`, `netloader`) → matching `crate-*` guidelines (if any) + relevant core

### Design Principles (special case)

When the user asks about design principles, or invokes `/code-guidelines principles`, load every `docs/code/principle-*.md` file and summarize.

## Proactive Loading

Load automatically based on the current task:

- Editing code → relevant core guidelines (error-handling, modules, types)
- Adding logs → `logging`
- Writing tests → `test-functions`, `test-files`, `test-organization`
- Documenting → `rust-documentation`
- Creating crates → `rust-workspace`, `rust-crate`
- Crate-specific work → matching `crate-*` guidelines + related core

Load multiple guidelines together when a task spans concerns (e.g. error handling + logging, crate-specific + core).

## When NOT to Use

- User needs to run commands → appropriate skill (`/code-format`, `/code-check`, `/code-test`)
- Guidelines already loaded in context → don't reload
