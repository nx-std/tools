---
name: "rust-documentation"
description: "Rustdoc patterns, safety documentation, function docs. Load when documenting code or writing docs"
type: core
scope: "global"
---

# Documentation Patterns

**🚨 MANDATORY for ALL documentation in this workspace**

## 🎯 PURPOSE

This document establishes consistent, succinct documentation standards across this entire codebase. These patterns ensure:

- **Rustdoc generation** - All public APIs documented for `cargo doc`
- **Succinct clarity** - Concise documentation that adds value without verbosity
- **Safety guarantees** - Explicit safety documentation for all unchecked operations
- **Clear feature contracts** - Well-documented Cargo features and error types

## 📑 TABLE OF CONTENTS

1. [Core Principles](#-core-principles)
   - [1. Succinct Documentation Philosophy](#1-succinct-documentation-philosophy)
   - [2. Document Safety-Critical Information](#2-document-safety-critical-information)
   - [3. Add Value Beyond Code](#3-add-value-beyond-code)
2. [Function Documentation Requirements](#-function-documentation-requirements)
   - [1. Brief Description Required](#1-brief-description-required)
   - [2. Document Key Behaviors](#2-document-key-behaviors)
3. [Safety Documentation (Mandatory)](#-safety-documentation-mandatory)
   - [1. Safety Section in `unsafe fn`](#1-safety-section-in-unsafe-fn)
   - [2. Safety Section in `_unchecked` Functions](#2-safety-section-in-_unchecked-functions)
   - [3. Safety Comments at Callsites](#3-safety-comments-at-callsites)
4. [Panics Documentation](#-panics-documentation)
5. [FFI Documentation](#-ffi-documentation)
   - [1. FFI Function Documentation](#1-ffi-function-documentation)
   - [2. C Type Documentation](#2-c-type-documentation)
6. [Error Documentation Requirements](#-error-documentation-requirements)
   - [1. Error Enum Documentation Template](#1-error-enum-documentation-template)
   - [2. Error Variant Documentation](#2-error-variant-documentation)
7. [Cargo.toml Feature Documentation](#-cargotoml-feature-documentation)
8. [Complete Examples](#-complete-examples)
9. [Checklist](#-checklist)

## 📐 CORE PRINCIPLES

### 1. Succinct Documentation Philosophy

**BE SUCCINCT**: Write concise documentation that adds value. Code should be self-documenting, but rustdocs need the obvious info for `cargo doc` generation. Keep it brief and clear.

```rust
// ❌ WRONG - Overly verbose
/// This function retrieves a user from the database by looking up their unique
/// identifier. It will search through all users and return the one matching
/// the provided ID, or None if no such user exists in the system.
pub fn get_user_by_id(id: UserId) -> Option<User> {
    // ...
}

// ✅ CORRECT - Succinct with value-added info
/// Retrieves a user by ID. Returns None if user was deleted or never existed.
pub fn get_user_by_id(id: UserId) -> Option<User> {
    // ...
}
```

### 2. Document Safety-Critical Information

**ALWAYS** document safety requirements, invariants, and preconditions. This is non-negotiable for `_unchecked` functions and unsafe code.

### 3. Add Value Beyond Code

Documentation should explain **behavior, edge cases, and important details** that aren't immediately obvious from the signature.

```rust
// ❌ WRONG - Merely repeating signature
/// Creates a new dataset with the given name and config
pub fn create_dataset(name: DatasetName, config: Config) -> Result<Dataset, Error> {
    // ...
}

// ✅ CORRECT - Documenting important behavior
/// Creates a new dataset. Idempotent - returns existing dataset if name already exists.
pub fn create_dataset(name: DatasetName, config: Config) -> Result<Dataset, Error> {
    // ...
}
```

## 📝 FUNCTION DOCUMENTATION REQUIREMENTS

### 1. Brief Description Required

**REQUIRED**: Every public function must have a brief description. Keep it to one or two sentences maximum.

```rust
// ✅ CORRECT - Brief, informative description
/// Processes a batch of records and updates the checkpoint atomically.
pub async fn process_batch(batch: RecordBatch) -> Result<(), Error> {
    // ...
}

// ✅ ALSO CORRECT - With important behavioral note
/// Connects to the database with automatic retry on transient failures.
/// Maximum retry attempts configured via `max_retries` parameter.
pub async fn connect_with_retry(url: &str, max_retries: u32) -> Result<Pool, Error> {
    // ...
}
```

### 2. Document Key Behaviors

**RECOMMENDED**: Document important behaviors, edge cases, or non-obvious details succinctly.

```rust
// ✅ CORRECT - Succinct with key behavior noted
/// Inserts a record into the database. Returns error if table doesn't exist.
pub async fn insert(pool: &Pool, record: Record, table_name: &str) -> Result<()> {
    // ...
}

// ✅ ALSO CORRECT - Documenting transaction behavior
/// Inserts multiple records in a single transaction. All-or-nothing semantics.
pub async fn insert_batch(pool: &Pool, records: Vec<Record>) -> Result<()> {
    // ...
}

// ❌ WRONG - Overly verbose parameter documentation
/// Inserts a record into the database
///
/// # Arguments
/// * `pool` - The database connection pool used for executing the insert
/// * `record` - The record data structure containing all fields to be inserted
/// * `table_name` - The name of the database table where the record will be stored
pub async fn insert(pool: &Pool, record: Record, table_name: &str) -> Result<()> {
    // ...
}
```

### 3. No Returns Section

**FORBIDDEN**: Do not include `# Returns` sections. Return types are self-documenting.

```rust
// ❌ WRONG - Unnecessary returns section
/// Gets the current configuration
///
/// # Returns
/// Returns the current configuration
pub fn get_config() -> Config {
    // ...
}

// ✅ CORRECT - No returns section
/// Gets the current configuration
pub fn get_config() -> Config {
    // ...
}
```

### 4. No Arguments Section

**FORBIDDEN**: Do not include `# Arguments` sections. Parameter names and types are self-documenting; put non-obvious constraints in the description.

```rust
// ❌ WRONG - Unnecessary arguments section
/// Creates a thread with the given parameters.
///
/// # Arguments
/// * `entry` - The thread entry point function
/// * `priority` - Thread priority (0-63)
/// * `core_id` - Preferred CPU core
pub fn create_thread(entry: ThreadFunc, priority: i32, core_id: i32) -> Result<ThreadHandle, CreateThreadError> { /* ... */ }

// ✅ CORRECT - Key info in description, no arguments section
/// Creates a thread. Priority must be 0-63; core_id of -2 uses default affinity.
pub fn create_thread(entry: ThreadFunc, priority: i32, core_id: i32) -> Result<ThreadHandle, CreateThreadError> { /* ... */ }
```

### 5. No Examples Section

**FORBIDDEN**: Do not include `# Examples` or usage examples sections in documentation. Tests serve as examples.

````rust
// ❌ WRONG - Unnecessary examples section
/// Validates a dataset name
///
/// # Examples
/// ```
/// let name = "my_dataset";
/// assert!(validate_name(name).is_ok());
/// ```
pub fn validate_name(name: &str) -> Result<()> {
    // ...
}

// ✅ CORRECT - No examples section
/// Validates a dataset name according to naming rules
pub fn validate_name(name: &str) -> Result<()> {
    // ...
}
````

## 🔒 SAFETY DOCUMENTATION (MANDATORY)

### 1. Safety Section in `unsafe fn`

**MANDATORY**: All `unsafe fn` declarations MUST include a `# Safety` section enumerating the caller's responsibilities.

```rust
// ✅ CORRECT - Safety section in unsafe function
/// Reads a value from the given memory address.
///
/// # Safety
/// - `addr` must be properly aligned for type `T`
/// - `addr` must point to a valid, initialized value of type `T`
/// - The memory must not be concurrently modified
pub unsafe fn read_volatile<T>(addr: *const T) -> T { /* ... */ }

// ❌ WRONG - Missing safety section
pub unsafe fn read_volatile<T>(addr: *const T) -> T { /* ... */ }
```

### 2. Safety Section in `_unchecked` Functions

**MANDATORY**: All functions with `_unchecked` suffix MUST include a `# Safety` section explaining the caller's responsibilities.

```rust
// ✅ CORRECT - Safety section in _unchecked function
/// Creates a dataset name wrapper from a string without validation
///
/// # Safety
/// The caller must ensure the provided name upholds dataset name invariants:
/// - Must not be empty
/// - Must contain only lowercase letters, numbers, underscores, and hyphens
/// - Must start with a letter
/// - Must not exceed 255 characters
///
/// Failure to uphold these invariants may cause undefined behavior in database operations.
pub fn from_str_unchecked(name: &str) -> DatasetName {
    // ...
}

// ❌ WRONG - Missing safety section
pub fn from_str_unchecked(name: &str) -> DatasetName {
    // ...
}
```

**Safety Section Template:**

```rust
/// # Safety
/// The caller must ensure [specific invariants/preconditions].
///
/// [Optional: Detailed explanation of consequences if invariants violated]
```

### 3. Safety Comments at Callsites

**MANDATORY**: All `unsafe { }` blocks and callsites of `_unchecked` functions (except in test code) MUST be preceded by a `// SAFETY:` comment explaining why the call upholds the documented invariants.

```rust
// ✅ CORRECT - SAFETY comment at unsafe block
let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
// SAFETY: ipc_buf points to the thread-local IPC buffer which is always
// valid and properly aligned for IPC operations.
unsafe { cmif::make_request(ipc_buf, cmd_id, 0) };

// ❌ WRONG - Missing SAFETY comment
let ipc_buf = nx_sys_thread_tls::ipc_buffer_ptr();
unsafe { cmif::make_request(ipc_buf, cmd_id, 0) };
```

The same applies to `_unchecked` callsites:

```rust
// ✅ CORRECT - SAFETY comment at callsite
let raw_name = fetch_from_database(&pool, id).await?;
// SAFETY: Database values are trusted to uphold invariants; validation occurs at boundaries before insertion.
let dataset_name = DatasetName::from_str_unchecked(&raw_name);

// ❌ WRONG - Missing SAFETY comment
let raw_name = fetch_from_database(&pool, id).await?;
let dataset_name = DatasetName::from_str_unchecked(&raw_name);
```

**Exception**: Test code does not require `// SAFETY:` comments, as tests are explicitly for exercising code with known inputs.

```rust
// ✅ CORRECT - Test code without SAFETY comments
#[test]
fn test_dataset_creation() {
    let name = DatasetName::from_str_unchecked("test_dataset");
    assert_eq!(name.as_str(), "test_dataset");
}
```

## ⚠️ PANICS DOCUMENTATION

**MANDATORY**: If a function can panic (uses `.unwrap()`, `.expect()`, `panic!()`, or calls functions that panic), it MUST include a `# Panics` section.

```rust
// ✅ CORRECT - Panics section for function that can panic
/// Extracts the maximum block number from the given ranges
///
/// # Panics
/// Panics if the ranges slice is empty
pub fn max_block_number(ranges: &[BlockRange]) -> u64 {
    ranges.iter().map(|r| r.end).max().unwrap()
}
```

**Panics Section Template:**

```rust
/// # Panics
/// Panics if [condition that causes panic]
```

## 🔌 FFI DOCUMENTATION

### 1. FFI Function Documentation

**REQUIRED**: All `pub extern "C"` functions in `ffi` modules must document:

- What the function does
- Safety requirements (always unsafe from the C perspective)
- Return value / error code conventions

```rust
// ✅ CORRECT - FFI function with full documentation
/// Creates a new thread.
///
/// # Safety
/// - `entry` must be a valid function pointer
/// - `stack_top` must point to a valid stack with sufficient size
/// - `out_handle` must be a valid pointer to write the result
///
/// Returns 0 on success, or a raw result code on failure.
/// On success, the new thread handle is written to `out_handle`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __nx_thread_create(
    out_handle: *mut u32,
    entry: ThreadFunc,
    arg: usize,
    stack_top: *mut c_void,
    priority: i32,
    core_id: i32,
) -> u32 { /* ... */ }
```

### 2. C Type Documentation

**REQUIRED**: Document C-compatible types with their intended usage and any invariants.

```rust
// ✅ CORRECT - C type with documentation
/// Thread entry function signature for C callers.
///
/// The function receives a single `usize` argument and should not return
/// (call `__nx_thread_exit` instead).
pub type ThreadFunc = unsafe extern "C" fn(arg: usize);
```

## 💥 ERROR DOCUMENTATION REQUIREMENTS

### 1. Error Enum Documentation Template

**MANDATORY**: All error enums MUST follow the documentation template from [errors-reporting.md](./errors-reporting.md).

```rust
// ✅ CORRECT - Comprehensive error enum documentation
/// Errors that occur during manifest registration operations
///
/// This enum represents all possible error conditions when registering
/// a new manifest in the dataset store.
#[derive(Debug, thiserror::Error)]
pub enum RegisterManifestError {
    /// Failed to store manifest in dataset definitions store
    ///
    /// This occurs when the object store cannot persist the manifest file.
    ///
    /// Possible causes:
    /// - Object store unavailable or unreachable
    /// - Insufficient permissions to write to object store
    /// - Network connectivity issues
    /// - Disk space exhausted on object store backend
    #[error("Failed to store manifest in dataset definitions store")]
    ManifestStorage(#[source] StoreError),

    /// Failed to register manifest in metadata database
    ///
    /// This occurs when the database operation fails during manifest registration.
    ///
    /// Possible causes:
    /// - Database connection lost
    /// - Constraint violation (duplicate manifest hash)
    /// - Transaction conflict with concurrent operations
    #[error("Failed to register manifest in metadata database")]
    MetadataRegistration(#[source] metadata_db::Error),
}
```

### 2. Error Variant Documentation

**MANDATORY**: Each error variant must include:

1. **Brief description** - One-line summary of what this error represents
2. **Detailed explanation** - When this error occurs
3. **Common causes** (optional) - Bullet list of typical causes
4. **Additional context** (optional) - Recovery strategies, transaction guarantees, etc.

**See [errors-reporting.md](./errors-reporting.md#11-error-documentation-template) for complete requirements.**

## ⚙️ CARGO.TOML FEATURE DOCUMENTATION

**MANDATORY**: Every feature in `Cargo.toml` MUST have a comment above it explaining its purpose.

```toml
# ✅ CORRECT - Feature with comment documentation
[features]
# Default features that are always enabled (unless default-features is set to false)
default = []
# Expose C-FFI symbols consumed by the nx-std umbrella crate
ffi = []
# High-level synchronization primitives built on nx-sys-sync
std-sync = ["dep:nx-sys-sync"]
```

```toml
# ❌ WRONG - Features without documentation
[features]
default = []
ffi = []
std-sync = ["dep:nx-sys-sync"]
```

**See [rust-crate.md](./rust-crate.md#2-features-section-rules) for complete requirements.**

## 📚 COMPLETE EXAMPLES

### Example 1: Succinct Function Documentation

```rust
/// Connects to the database with automatic retry on transient failures.
pub async fn connect_with_retry(url: &str, max_retries: u32) -> Result<Pool, Error> {
    // Implementation
}

/// Lists all active jobs for the given worker. Returns empty vec if worker has no jobs.
pub async fn list_active_jobs(worker_id: WorkerId) -> Result<Vec<Job>, Error> {
    // Implementation
}

/// Deletes the dataset and all associated files. Idempotent - returns Ok if already deleted.
pub async fn delete_dataset(name: &DatasetName) -> Result<(), Error> {
    // Implementation
}
```

### Example 2: Function with Safety Documentation

```rust
/// Creates a manifest hash wrapper from a hexadecimal string without validation
///
/// # Safety
/// The caller must ensure the provided string is exactly 64 hexadecimal characters.
/// Invalid input may cause database constraint violations or query failures.
pub fn from_hex_unchecked(hex: &str) -> ManifestHash {
    ManifestHash(hex.to_string())
}

// Usage with SAFETY comment
pub fn load_manifest(hash_str: &str) -> Result<Manifest, Error> {
    validate_hex_hash(hash_str)?;
    // SAFETY: validate_hex_hash ensures the string is exactly 64 hex characters
    let hash = ManifestHash::from_hex_unchecked(hash_str);
    Ok(Manifest { hash })
}
```

### Example 3: Function with Panics Documentation

```rust
/// Extracts the last block number from a non-empty range list
///
/// # Panics
/// Panics if ranges is empty
pub fn last_block_number(ranges: &[BlockRange]) -> u64 {
    ranges.last().unwrap().end
}
```

### Example 4: Error Enum Documentation

```rust
/// Errors that occur during checkpoint operations
#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    /// Failed to save checkpoint to database
    ///
    /// This occurs when the database operation fails during checkpoint persistence.
    /// The checkpoint update is atomic - either fully saved or not saved at all.
    ///
    /// Possible causes:
    /// - Database connection lost
    /// - Transaction conflict with concurrent checkpoint updates
    /// - Disk space exhausted
    ///
    /// Recovery: The operation can be safely retried as checkpoints are idempotent.
    #[error("Failed to save checkpoint")]
    SaveFailed(#[source] sqlx::Error),
}
```

### Example 5: Cargo.toml Feature Documentation

```toml
[features]
# Default features that are always enabled (unless default-features is set to false)
default = ["tls-support"]
# TLS support for secure database connections
tls-support = ["dep:rustls", "dep:tokio-rustls"]
# Redis caching support with tokio async client
redis-cache = ["dep:redis", "dep:tokio"]
# Metrics collection and Prometheus export
metrics = ["dep:metrics", "dep:metrics-prometheus"]
```

## ✅ CHECKLIST

Before committing code, verify:

### Function Documentation

- [ ] All public functions have succinct documentation (1-2 sentences max)
- [ ] Documentation includes key behaviors and edge cases
- [ ] No `# Arguments`, `# Returns`, or `# Examples` sections
- [ ] Documentation adds value beyond what the signature conveys

### Safety Documentation

- [ ] All `unsafe fn` have `# Safety` section in rustdocs
- [ ] All `_unchecked` functions have `# Safety` section in rustdocs
- [ ] All `unsafe { }` blocks and `_unchecked` callsites (except tests) have `// SAFETY:` comments
- [ ] Safety comments explain why the operation is safe

### FFI Documentation

- [ ] All `pub extern "C"` functions are documented
- [ ] FFI safety requirements are documented
- [ ] Return value / error code conventions are documented

### Panics Documentation

- [ ] Functions that can panic have `# Panics` section
- [ ] Panic conditions are clearly documented

### Error Documentation

- [ ] Error enums follow template from errors-reporting.md
- [ ] Each error variant has comprehensive documentation
- [ ] Common causes listed for each variant (when applicable)

### Cargo.toml Feature Documentation

- [ ] Every feature has comment documentation above it
- [ ] Feature descriptions are clear and concise

---

**🚨 CRITICAL**: These documentation standards are MANDATORY and must be followed for all new code and when modifying existing code. 🚫 **NO EXCEPTIONS**.
