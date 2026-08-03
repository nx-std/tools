//! The `[package.metadata.nx]` blocks a package declares, as they are written in
//! `Cargo.toml`.
//!
//! These types mirror the manifest rather than the formats they end up in: every
//! field is optional or defaulted, because a manifest is user-authored and may omit
//! anything. Turning them into format inputs, and rejecting what cannot be turned,
//! is the job of the sibling modules that consume them.
//!
//! Paths are written relative to the package root and resolved against it, never
//! against the working directory the build was invoked from.

/// The `[package.metadata.nx.nsp]` block: how to obtain the process descriptor an
/// NSP must carry.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NspMetadata {
    /// Path to a JSON descriptor, relative to the package root.
    ///
    /// Used only when `npdm` is absent; declaring both silently ignores this one.
    pub npdm_json: Option<String>,
    /// The descriptor written inline in the manifest.
    ///
    /// Takes precedence over `npdm_json`. Declaring neither fails the build, since
    /// an NSP cannot be assembled without one.
    pub npdm: Option<InlineNpdm>,
}

/// The `[package.metadata.nx.nsp.npdm]` block: a process descriptor written in TOML
/// rather than supplied as a JSON file.
///
/// Every field here is required, unlike the rest of the manifest surface: a
/// descriptor with a missing field cannot be completed by defaulting, because the
/// values describe what the kernel will grant the process.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct InlineNpdm {
    /// Process name recorded in the descriptor.
    pub name: String,
    /// Stack size for the main thread, as hexadecimal.
    ///
    /// A `0x` prefix is accepted and stripped before the descriptor is written.
    pub main_thread_stack_size: String,
    /// Scheduling priority of the main thread.
    pub main_thread_priority: u8,
    /// Core the main thread is pinned to.
    ///
    /// Accepts `default_cpu_id` as well, which is the name the JSON descriptor
    /// uses for the same value.
    #[serde(alias = "main_thread_core_number", alias = "default_cpu_id")]
    pub main_thread_core_number: u8,
    /// Process version, written to the descriptor as hexadecimal.
    ///
    /// Defaults to zero, which is what a title that does not version itself uses.
    #[serde(default)]
    pub version: u32,
    /// Address space layout the process is loaded under.
    pub address_space_type: u8,
    /// Whether the process runs as 64-bit.
    pub is_64_bit: bool,
    /// Whether the kernel may optimize the process's memory allocation.
    pub optimize_memory_allocation: bool,
    /// Whether to keep device address spaces separate rather than merged.
    pub disable_device_address_space_merge: bool,
    /// Whether the descriptor is signed for retail rather than development.
    ///
    /// Defaults to `true`, matching what a title built for a console expects.
    #[serde(default = "default_is_retail")]
    pub is_retail: bool,
    /// Program id, as hexadecimal.
    ///
    /// A `0x` prefix is accepted and stripped. Also becomes both ends of the
    /// descriptor's permitted program-id range, so the process may only ever run
    /// under this exact id. Accepts `title_id` as an alias.
    #[serde(alias = "title_id")]
    pub program_id: String,
    /// Filesystem permissions granted to the process.
    ///
    /// Omitting the block grants nothing, rather than leaving the field unset: the
    /// descriptor requires a value.
    #[serde(default)]
    pub fs_access_control: Option<InlineFsAccessControl>,
    /// Services the process may access or provide.
    ///
    /// Omitting the block declares neither.
    #[serde(default)]
    pub service_access_control: Option<InlineServiceAccessControl>,
    /// Kernel capabilities requested by the process.
    ///
    /// Omitting the block requests none, and the descriptor carries an empty
    /// capability list.
    #[serde(default)]
    pub kernel_capabilities: Option<InlineKernelCapabilities>,
}

fn default_is_retail() -> bool {
    true
}

/// The `fs_access_control` sub-block: what the process may do to the filesystem.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct InlineFsAccessControl {
    /// Permission bitmask, as hexadecimal.
    ///
    /// A `0x` prefix is accepted and stripped.
    pub flags: String,
}

/// The `service_access_control` sub-block: the services a process talks to, and the
/// ones it answers for.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct InlineServiceAccessControl {
    /// Services the process may connect to as a client.
    #[serde(default)]
    pub accessed_services: Vec<String>,
    /// Services the process registers and serves itself.
    #[serde(default)]
    pub hosted_services: Vec<String>,
}

/// The `kernel_capabilities` sub-block: what the process asks the kernel to permit.
///
/// The thread-priority and core bounds share one emitted capability: setting any of
/// the four produces it, carrying only those that were set. Leaving all four unset
/// omits the capability entirely.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct InlineKernelCapabilities {
    /// Numerically highest thread priority the process may request.
    #[serde(default)]
    pub highest_priority: Option<u8>,
    /// Numerically lowest thread priority the process may request.
    #[serde(default)]
    pub lowest_priority: Option<u8>,
    /// Highest core index the process may run threads on.
    #[serde(default)]
    pub max_core_number: Option<u8>,
    /// Lowest core index the process may run threads on.
    #[serde(default)]
    pub min_core_number: Option<u8>,
    /// Syscalls the process is permitted to make, by name.
    ///
    /// Names must match the kernel ABI exactly; one that does not fails the build
    /// rather than being dropped from the descriptor.
    #[serde(default)]
    pub enable_system_calls: Vec<String>,
    /// Minimum kernel version the process requires.
    ///
    /// Accepted either as `major.minor`, which is encoded here, or as a bare hex
    /// string, which is passed through unchanged.
    #[serde(default)]
    pub kernel_version: Option<String>,
}

/// The `[package.metadata.nx.nro.nacp]` block: the control data describing a title
/// to the console.
///
/// Every field is optional; the ones that are absent take a placeholder default
/// rather than failing the build, so an NRO always carries usable control data.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NacpMetadata {
    /// Title name shown on the console, for every language without its own entry.
    pub name: Option<String>,
    /// Author shown on the console, for every language without its own entry.
    pub author: Option<String>,
    /// Display version string.
    pub version: Option<String>,
    /// Application id, as hexadecimal.
    ///
    /// A `0x` prefix is *not* accepted here, unlike the NPDM fields: the value is
    /// read as bare hex digits and a prefix fails the build.
    pub title_id: Option<String>,
    /// Base id for downloadable content.
    ///
    /// Accepted by the manifest but not yet applied: the NACP builder derives the
    /// add-on content base id from `title_id` instead, so setting this has no
    /// effect on the output.
    pub dlc_base_title_id: Option<String>,
    /// Per-language overrides of `name` and `author`.
    ///
    /// A language absent from the table falls back to the global pair rather than
    /// being left blank.
    pub lang: Option<NacpLangEntries>,
}

/// Per-language NACP entries.
///
/// The keys are written in the manifest as the console spells them, so the hyphen
/// and case are significant: `en-US`, not `en_us`.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct NacpLangEntries {
    /// American English.
    #[serde(rename = "en-US")]
    pub en_us: Option<NacpLangEntry>,
    /// British English.
    #[serde(rename = "en-GB")]
    pub en_gb: Option<NacpLangEntry>,
    /// Japanese.
    pub ja: Option<NacpLangEntry>,
    /// French.
    pub fr: Option<NacpLangEntry>,
    /// German.
    pub de: Option<NacpLangEntry>,
    /// Latin American Spanish.
    #[serde(rename = "es-419")]
    pub es_419: Option<NacpLangEntry>,
    /// Spanish.
    pub es: Option<NacpLangEntry>,
    /// Italian.
    pub it: Option<NacpLangEntry>,
    /// Dutch.
    pub nl: Option<NacpLangEntry>,
    /// Canadian French.
    #[serde(rename = "fr-CA")]
    pub fr_ca: Option<NacpLangEntry>,
    /// Portuguese.
    pub pt: Option<NacpLangEntry>,
    /// Russian.
    pub ru: Option<NacpLangEntry>,
    /// Korean.
    pub ko: Option<NacpLangEntry>,
    /// Traditional Chinese.
    #[serde(rename = "zh-TW")]
    pub zh_tw: Option<NacpLangEntry>,
    /// Simplified Chinese.
    #[serde(rename = "zh-CN")]
    pub zh_cn: Option<NacpLangEntry>,
    /// Brazilian Portuguese.
    #[serde(rename = "pt-BR")]
    pub pt_br: Option<NacpLangEntry>,
}

/// A single language's title and author.
///
/// Both are required once a language is declared: an entry that overrode only one
/// of them would leave the other silently falling back, which is harder to read in
/// a manifest than repeating it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NacpLangEntry {
    /// Title name for this language.
    pub name: String,
    /// Author for this language.
    pub author: String,
}

/// The `[package.metadata.nx.nro]` block: what an NRO carries besides the
/// executable.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NroMetadata {
    /// Directory to pack as the RomFS image, relative to the package root.
    ///
    /// Omitting it produces an NRO with no filesystem attached.
    pub romfs: Option<String>,
    /// Icon file, relative to the package root.
    ///
    /// Omitting it embeds a built-in placeholder, so an NRO always has an icon.
    pub icon: Option<String>,
    /// Control data describing the title.
    ///
    /// Omitting it produces an NRO with no control data, which the console will
    /// launch but cannot describe.
    pub nacp: Option<NacpMetadata>,
    /// Whether to build an overlay rather than an ordinary NRO.
    ///
    /// Changes only the output file's extension, to `.ovl`; the bytes are the same
    /// either way.
    pub overlay: Option<bool>,
}
