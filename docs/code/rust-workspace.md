---
name: "rust-workspace"
description: "Workspace crate categories, dependency rules, member ordering. Load when creating crates, managing workspace structure, or reviewing dependency direction"
type: arch
scope: "global"
---

# Rust Workspace Patterns

**MANDATORY for ALL workspace-level organization in this workspace**

## Table of Contents

1. [Crate Category Hierarchy](#1-crate-category-hierarchy)
2. [Dependency Rules per Category](#2-dependency-rules-per-category)
3. [Ordering Requirements](#3-ordering-requirements)
4. [Checklist](#checklist)

## 1. Crate Category Hierarchy

This workspace is a Meson-based monorepo. Cargo workspace members live under `subprojects/<crate>/` and are split into the following architectural layers, from foundation to leaf:

### `sys/*` — Foundation Layer

**Purpose**: Direct interface to Horizon OS primitives. Foundation for everything else.

**What belongs here:**

- **`nx-svc`**: Raw Supervisor Call (SVC) bindings — the layer everything else depends on.
- **`nx-cpu`**: CPU-level utilities (cache, registers).
- **`nx-sys-mem`**: Low-level memory management on top of `nx-svc`.
- **`nx-sys-sync`**: Low-level synchronization primitives on top of `nx-svc`.
- **`nx-sys-thread`**, **`nx-sys-thread-tls`**: Thread management.

### Higher-level Crates — Standard-library-style abstractions

**Purpose**: `std`-flavoured abstractions built on the `sys/*` layer.

**What belongs here:**

- **`nx-alloc`**: Global allocator (uses `nx-svc` + `nx-sys-sync`).
- **`nx-rand`**: Random number generation.
- **`nx-time`**: Time utilities.
- **`nx-std-sync`**: High-level sync primitives (`Mutex`, `RwLock`, …).
- **`nx-rt`**: Runtime support.
- **`nx-panic-handler`**: Panic handler.

### Service Crates (`nx-service-*`) — Horizon OS Services

**Purpose**: Bindings to specific Horizon OS services exposed via IPC.

**What belongs here:**

- **`nx-sf`**: Service framework primitives.
- **`nx-service-sm`**, **`nx-service-time`**, **`nx-service-applet`**, **`nx-service-hid`**, **`nx-service-vi`**, **`nx-service-set`**, **`nx-service-apm`**, **`nx-service-nv`**: Per-service IPC clients.

### `nx-std` — Umbrella Staticlib

**Purpose**: Single `staticlib` crate that re-exports the C-FFI symbols (`__nx_*`) consumed by linker overrides. Each enabled higher-level / `sys/*` / service crate exposes its FFI surface via a public `ffi` module behind an `ffi` Cargo feature; `nx-std` re-exports them based on enabled features.

This is the only crate that produces a linkable artifact for the C side.

### `subprojects/tests/`

The Switch-hardware NRO test suite. C code linking against the Rust crates to verify FFI correctness.

## 2. Dependency Rules per Category

| From \ To              | `sys/*` | higher-level | service | `nx-std` |
|------------------------|:-------:|:------------:|:-------:|:--------:|
| **`sys/*`**            | ✅       | ❌            | ❌       | ❌        |
| **higher-level**       | ✅       | ✅            | ❌       | ❌        |
| **service**            | ✅       | ✅            | ✅       | ❌        |
| **`nx-std`** umbrella  | ✅       | ✅            | ✅       | ❌        |

**Key rules:**

- **`sys/*` crates** depend only on other `sys/*` crates and `nx-svc`. They NEVER depend on higher-level, service, or umbrella crates.
- **Higher-level crates** depend on `sys/*` and other higher-level crates. They MUST NOT depend on service or umbrella crates.
- **Service crates** depend on `sys/*`, higher-level, and other service crates as needed (e.g., service-applet depends on service-sm). They MUST NOT depend on the `nx-std` umbrella.
- **`nx-std`** is the sink: every other crate may flow into it; nothing depends on `nx-std`.
- **No circular dependencies** at any layer.
- The `ffi` feature on each crate gates its C-FFI module; `nx-std` enables exactly those `ffi` features that match the Meson `use_nx*` setup-time options.

## 3. Ordering Requirements

### Workspace Members

The root `Cargo.toml` `members` array MUST be ordered alphabetically.

**Rationale**: Ensures consistent merge conflict resolution and predictable workspace member listing.

### Dependencies in Cargo.toml

All `Cargo.toml` dependency sections (`[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`) MUST be ordered alphabetically.

**Rationale**: Ensures consistent merge conflict resolution and maintainable dependency management.

## References

- [rust-crate](rust-crate.md) - Related: Crate manifest conventions

## Checklist

Before committing workspace changes, verify:

- [ ] New crates are placed in the correct architectural layer (`sys/*` foundation, higher-level, service, or umbrella).
- [ ] Dependency direction follows the rules (no upward edges, no cycles).
- [ ] Workspace `members` array is alphabetically ordered.
- [ ] All `Cargo.toml` dependency sections are alphabetically ordered.
- [ ] `sys/*` crates have no dependencies on higher-level, service, or `nx-std` crates.
- [ ] If the crate exposes a C-FFI surface, it lives behind an `ffi` Cargo feature and is re-exported by `nx-std` when the corresponding `use_nx*` Meson option is enabled.
