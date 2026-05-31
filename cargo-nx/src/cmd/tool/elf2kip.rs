use std::{
    fs,
    io::{self, BufWriter},
    path::PathBuf,
};

use nx_object::{
    elf::{self, ElfSegments},
    write::{kip, npdm::KernelCapability},
};

use crate::npdm::{self, KernelCapabilityDescriptor, U64OrHex};

pub fn handle_subcommand(args: Args) -> Result<(), Error> {
    // Load and parse ELF file
    let elf_data = fs::read(&args.elf_file).map_err(|err| Error::ReadElf {
        path: args.elf_file.clone(),
        source: err,
    })?;

    let segments = ElfSegments::parse(&elf_data).map_err(|err| Error::ParseElf {
        path: args.elf_file.clone(),
        source: err,
    })?;

    // Parse JSON descriptor
    let json_data = fs::read_to_string(&args.json_file).map_err(|err| Error::ReadJson {
        path: args.json_file.clone(),
        source: err,
    })?;

    let descriptor: KipDescriptor =
        serde_json::from_str(&json_data).map_err(|err| Error::ParseJson {
            path: args.json_file.clone(),
            source: err,
        })?;

    // Convert the descriptor kernel capabilities into their nx-object form,
    // validating ranges and lowering byte addresses to page numbers.
    let kernel_capabilities = descriptor
        .kernel_capabilities
        .into_iter()
        .map(KernelCapability::try_from)
        .collect::<Result<Vec<KernelCapability>, _>>()
        .map_err(Error::KernelCapability)?;

    // Encode kernel capabilities
    let capabilities: Vec<u32> = kernel_capabilities
        .iter()
        .flat_map(|cap| cap.encode())
        .collect();

    // Build KIP1 using nx-object builder
    let mut builder = nx_object::write::Kip1Builder::new()
        .name(descriptor.name)
        .title_id(descriptor.title_id.get())
        .process_category(descriptor.process_category.get() as u32)
        .main_thread_priority(descriptor.main_thread_priority)
        .default_cpu_id(descriptor.default_cpu_id)
        .rodata_attributes(descriptor.main_thread_stack_size.get() as u32)
        .kernel_capabilities(capabilities);

    // Compute flags: raw override takes precedence, otherwise use boolean fields.
    // use_secure_memory controls bit 5, immortal controls bit 6.
    if let Some(flags) = descriptor.flags {
        // Raw flags override: use as-is
        builder = builder.flags(flags);
    } else {
        // Compute flags from boolean fields
        // Start with base flags 0x3F:
        // bits 0-5 set (compression + Is64Bit + IsAddrSpace32Bit + UseSystemPoolPartition)
        let mut flags = 0x3F;

        // Apply boolean modifiers
        if descriptor.use_secure_memory {
            flags |= 0x20; // Set bit 5
        } else {
            flags &= !0x20; // Clear bit 5
        }

        if descriptor.immortal {
            flags |= 0x40; // Set bit 6
        } else {
            flags &= !0x40; // Clear bit 6
        }

        builder = builder.flags(flags);
    }

    // Extract segments and build KIP1
    let text = segments.text();
    let text_vaddr = segments.text_vaddr();
    let rodata = segments.rodata();
    let rodata_vaddr = segments.rodata_vaddr();
    let data = segments.data();
    let data_vaddr = segments.data_vaddr();

    builder = builder
        .text(text.to_vec())
        .text_vaddr(text_vaddr as u32)
        .rodata(rodata.to_vec())
        .rodata_vaddr(rodata_vaddr as u32)
        .data(data.to_vec())
        .data_vaddr(data_vaddr as u32);

    // Add BSS if present
    let bss_size = segments.bss_size();
    if bss_size > 0 {
        // BSS vaddr follows data segment, page-aligned to 0x1000
        // The kernel maps segments at page granularity, so we must align the data
        // segment size before computing the BSS base address to prevent overlap
        let bss_vaddr = data_vaddr + ((data.len() as u64 + 0xFFF) & !0xFFF);
        builder = builder
            .bss_vaddr(bss_vaddr as u32)
            .bss_size(bss_size as u32);
    }

    // Build KIP1 binary
    let kip_data = builder.build().map_err(Error::BuildKip)?;

    // Write output file
    let output_file = fs::File::create(&args.kip_file).map_err(|err| Error::CreateOutput {
        path: args.kip_file.clone(),
        source: err,
    })?;

    let mut writer = BufWriter::new(output_file);
    std::io::Write::write_all(&mut writer, &kip_data).map_err(|err| Error::WriteOutput {
        path: args.kip_file.clone(),
        source: err,
    })?;

    Ok(())
}

#[derive(clap::Args)]
pub struct Args {
    /// Path to the input ELF file
    pub elf_file: PathBuf,

    /// Path to the JSON descriptor file
    pub json_file: PathBuf,

    /// Path to the output KIP file
    pub kip_file: PathBuf,
}

/// Errors from the `elf2kip` subcommand
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to read the input ELF file from disk
    #[error("Failed to read ELF file '{}'", path.display())]
    ReadElf { path: PathBuf, source: io::Error },

    /// Failed to parse ELF segments from the input file
    #[error("Failed to parse ELF file '{}'", path.display())]
    ParseElf {
        path: PathBuf,
        source: elf::ParseError,
    },

    /// Failed to read the JSON descriptor file from disk
    #[error("Failed to read JSON file '{}'", path.display())]
    ReadJson { path: PathBuf, source: io::Error },

    /// Failed to deserialize the KIP descriptor from JSON
    #[error("Failed to parse JSON file '{}'", path.display())]
    ParseJson {
        path: PathBuf,
        source: serde_json::Error,
    },

    /// Failed to convert the descriptor kernel capabilities
    #[error("Failed to build kernel capabilities from descriptor")]
    KernelCapability(#[source] npdm::Error),

    /// Failed to build the KIP binary
    #[error("Failed to build KIP")]
    BuildKip(#[source] kip::BuildError),

    /// Failed to create the output file
    #[error("Failed to create KIP output file '{}'", path.display())]
    CreateOutput { path: PathBuf, source: io::Error },

    /// Failed to write the KIP data to the output file
    #[error("Failed to write KIP file '{}'", path.display())]
    WriteOutput { path: PathBuf, source: io::Error },
}

/// JSON descriptor for KIP1 metadata.
#[derive(Debug, serde::Deserialize)]
struct KipDescriptor {
    name: String,

    #[serde(alias = "program_id")]
    title_id: U64OrHex,

    main_thread_stack_size: U64OrHex,

    main_thread_priority: u8,

    default_cpu_id: u8,

    #[serde(alias = "version", default = "default_process_category")]
    process_category: U64OrHex,

    /// Optional raw flags byte. When present, overrides the boolean flag fields.
    #[serde(default)]
    flags: Option<u8>,

    /// Controls bit 5 (UseSystemPoolPartition) of the flags byte; defaults to `true`.
    /// Ignored if the raw `flags` field is present.
    #[serde(default = "default_use_secure_memory")]
    use_secure_memory: bool,

    /// Controls bit 6 (Immortal) of the flags byte; defaults to `true`.
    /// Ignored if the raw `flags` field is present.
    #[serde(default = "default_immortal")]
    immortal: bool,

    /// Kernel capabilities, encoded into the KIP1 capability descriptors.
    kernel_capabilities: Vec<KernelCapabilityDescriptor>,
}

/// Default process category value.
fn default_process_category() -> U64OrHex {
    U64OrHex::from(1)
}

/// Default `use_secure_memory` value.
fn default_use_secure_memory() -> bool {
    true
}

/// Default `immortal` value.
fn default_immortal() -> bool {
    true
}
