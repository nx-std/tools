Output Format:
    Emits GAS-style assembly directives to stdout (or to PATH given via -o).
    Each input file produces a self-contained block with .global symbols
    for the array and its end, suitable for linking into C/C++ programs as
    `extern const uint8_t name[];` and `extern const uint8_t name_end[];`.
    A `name_size` symbol is also emitted unless --header is used, in which
    case the size lives in the generated header instead.

Symbol Naming:
    Symbols are derived from the input file's basename, sanitized to a
    valid identifier:
      - `-`, `.`, `/` are replaced with `_`
      - other non-alphanumeric, non-`_` characters are dropped
      - a leading `_` is prepended when the name starts with a digit,
        or always under --apple-llvm

    Example: `4bit.chr` produces `_4bit_chr`, `_4bit_chr_end`, and
    `_4bit_chr_size`. The companion C header always uses the
    non-prefixed form so it can be #included regardless of --apple-llvm.

Empty Inputs:
    Empty input files are skipped with a warning on stderr (visible with
    RUST_LOG=warn) and produce no output block.

Examples:
    # Embed a single binary blob (output to stdout)
    cargo nx tool bin2s assets/font.bin > assets/font.s

    # Write directly to a file
    cargo nx tool bin2s -o assets/font.s assets/font.bin

    # Multiple inputs concatenated into one assembly file
    cargo nx tool bin2s -o assets/embedded.s \
        assets/font.bin assets/sprite.bin

    # Generate a matching C header (size lives in the header, not the asm)
    cargo nx tool bin2s -H assets/embedded.h -o assets/embedded.s \
        assets/font.bin

    # Custom alignment for the .balign directive (default is 4)
    cargo nx tool bin2s -a 16 assets/aligned.bin

    # Apple LLVM assembler dialect (uses .const_data, prepends `_`)
    cargo nx tool bin2s --apple-llvm assets/blob.bin
