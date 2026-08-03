//! The `[package.metadata.nx]` blocks a package declares, as they are written in
//! `Cargo.toml`.
//!
//! These types mirror the manifest rather than the formats they end up in: every
//! field is optional or defaulted, because a manifest is user-authored and may omit
//! anything. Turning them into format inputs, and rejecting what cannot be turned,
//! is the job of the sibling modules that consume them.

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NspMetadata {
    pub npdm_json: Option<String>,
    pub npdm: Option<InlineNpdm>,
}

/// Inline NPDM metadata structure matching Cargo.toml format
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct InlineNpdm {
    pub name: String,
    pub main_thread_stack_size: String,
    pub main_thread_priority: u8,
    #[serde(alias = "main_thread_core_number", alias = "default_cpu_id")]
    pub main_thread_core_number: u8,
    #[serde(default)]
    pub version: u32,
    pub address_space_type: u8,
    pub is_64_bit: bool,
    pub optimize_memory_allocation: bool,
    pub disable_device_address_space_merge: bool,
    #[serde(default = "default_is_retail")]
    pub is_retail: bool,
    #[serde(alias = "title_id")]
    pub program_id: String,
    #[serde(default)]
    pub fs_access_control: Option<InlineFsAccessControl>,
    #[serde(default)]
    pub service_access_control: Option<InlineServiceAccessControl>,
    #[serde(default)]
    pub kernel_capabilities: Option<InlineKernelCapabilities>,
}

fn default_is_retail() -> bool {
    true
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct InlineFsAccessControl {
    pub flags: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct InlineServiceAccessControl {
    #[serde(default)]
    pub accessed_services: Vec<String>,
    #[serde(default)]
    pub hosted_services: Vec<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct InlineKernelCapabilities {
    #[serde(default)]
    pub highest_priority: Option<u8>,
    #[serde(default)]
    pub lowest_priority: Option<u8>,
    #[serde(default)]
    pub max_core_number: Option<u8>,
    #[serde(default)]
    pub min_core_number: Option<u8>,
    #[serde(default)]
    pub enable_system_calls: Vec<String>,
    #[serde(default)]
    pub kernel_version: Option<String>,
}

/// Serde-compatible NACP metadata that deserializes from Cargo.toml
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NacpMetadata {
    pub name: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub title_id: Option<String>,
    pub dlc_base_title_id: Option<String>,
    pub lang: Option<NacpLangEntries>,
}

/// Per-language NACP entries.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct NacpLangEntries {
    #[serde(rename = "en-US")]
    pub en_us: Option<NacpLangEntry>,
    #[serde(rename = "en-GB")]
    pub en_gb: Option<NacpLangEntry>,
    pub ja: Option<NacpLangEntry>,
    pub fr: Option<NacpLangEntry>,
    pub de: Option<NacpLangEntry>,
    #[serde(rename = "es-419")]
    pub es_419: Option<NacpLangEntry>,
    pub es: Option<NacpLangEntry>,
    pub it: Option<NacpLangEntry>,
    pub nl: Option<NacpLangEntry>,
    #[serde(rename = "fr-CA")]
    pub fr_ca: Option<NacpLangEntry>,
    pub pt: Option<NacpLangEntry>,
    pub ru: Option<NacpLangEntry>,
    pub ko: Option<NacpLangEntry>,
    #[serde(rename = "zh-TW")]
    pub zh_tw: Option<NacpLangEntry>,
    #[serde(rename = "zh-CN")]
    pub zh_cn: Option<NacpLangEntry>,
    #[serde(rename = "pt-BR")]
    pub pt_br: Option<NacpLangEntry>,
}

/// Single language entry with name and author
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NacpLangEntry {
    pub name: String,
    pub author: String,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NroMetadata {
    pub romfs: Option<String>,
    pub icon: Option<String>,
    pub nacp: Option<NacpMetadata>,
    pub overlay: Option<bool>,
}
