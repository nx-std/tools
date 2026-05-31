fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(gen_schema_npdm)]
    {
        println!("cargo:warning=Config 'gen_schema_npdm' enabled: Running JSON schema generation");
        let out_dir = std::env::var("OUT_DIR")?;
        let schema = schemars::schema_for!(cargo_nx::npdm::NpdmDescriptor);
        let schema_json = serde_json::to_string_pretty(&schema)?;
        let schema_path = format!("{out_dir}/schema.json");
        std::fs::write(&schema_path, schema_json)?;
        println!("cargo:warning=Generated NPDM schema file: {schema_path}");
    }
    #[cfg(not(gen_schema_npdm))]
    {
        println!(
            "cargo:debug=Config 'gen_schema_npdm' not enabled: Skipping JSON schema generation"
        );
    }

    Ok(())
}
