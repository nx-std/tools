Output Format:
    Emits a C source to stdout (or to PATH given via -o) containing a
    `const unsigned char name[]` array and a matching
    `const unsigned int name_size = sizeof(name);` constant for each input
    file. Suitable for direct inclusion in C/C++ build trees.

Symbol Naming:
    Symbols are derived from the input file's basename, sanitized to a
    valid C identifier:
      - `-`, `.`, `/` are replaced with `_`
      - other non-alphanumeric, non-`_` characters are dropped
      - a leading `_` is prepended when the name starts with a digit

    Example: `4bit.chr` produces `_4bit_chr` and `_4bit_chr_size`.

Header Generation:
    With -H/--header, a matching C header with extern declarations is
    written to PATH. The header uses `#pragma once` for include guarding.

Empty Inputs:
    Empty input files are skipped with a warning on stderr (visible with
    RUST_LOG=warn) and produce no output block.

Aliases:
    This command is also available as `raw2c` for compatibility with the
    devkitPro general-tools utility of the same name. Behavior is
    identical under either name.

Examples:
    # Embed a single binary blob (output to stdout)
    cargo nx tool bin2c assets/font.bin > assets/font.c

    # Write directly to a file
    cargo nx tool bin2c -o assets/font.c assets/font.bin

    # Multiple inputs concatenated into one C source file
    cargo nx tool bin2c -o assets/embedded.c \
        assets/font.bin assets/sprite.bin

    # Generate a matching C header alongside the source
    cargo nx tool bin2c -H assets/embedded.h -o assets/embedded.c \
        assets/font.bin

    # Use the raw2c alias
    cargo nx tool raw2c assets/font.bin > assets/font.c
