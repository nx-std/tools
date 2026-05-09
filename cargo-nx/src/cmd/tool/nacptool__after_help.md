Output Format:
    Builds a NACP (Nintendo Application Control Property) blob at OUTFILE.
    NACP is the control-data record carried inside an NRO Asset section
    or alongside an NSP, providing the user-visible application name,
    author, and version reported by the OS launcher.

Mode:
    --create selects creation mode and is currently the only supported
    mode (this matches the C `nacptool` interface). The flag is required
    so that future modes can be added without breaking existing callers.

Title ID:
    --titleid takes a 16-character hexadecimal application ID with no
    `0x` prefix. The format is validated strictly to match the C tool's
    sscanf("%016" SCNx64) behavior — fewer than 16 digits, more than 16
    digits, or non-hex characters are rejected. When omitted the
    application_id field is left at zero.

Examples:
    # Minimal NACP with name, author, and version
    cargo nx tool nacptool --create "MyApp" "Author Name" "1.0.0" control.nacp

    # Include an explicit title ID
    cargo nx tool nacptool --create "MyApp" "Author Name" "1.0.0" \
        control.nacp --titleid=0100000000010000
