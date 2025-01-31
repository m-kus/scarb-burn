# Scarb Burn

Scarb extension for generating Cairo flame charts and pprof profiles.

![image](https://github.com/user-attachments/assets/c94e3660-1c53-479b-b3b0-ad363cdb01fc)

This is a complementary tool to [cairo-profiler](https://github.com/software-mansion/cairo-profiler), focused on generic Cairo programs run outside of Starknet context. It is particularly useful if you have a program that consumes a large arguments file.

## Installation

You need to have Rust installed on your machine.

```bash
cargo install --git https://github.com/m-kus/scarb-burn scarb-burn
```

## Usage

Run in your project directory:

```bash
# Generate flamegraph (default)
scarb burn --arguments-file arguments.json --output-file flamegraph.svg --open-in-browser

# Generate pprof profile (requires Go toolchain for visualization)
scarb burn --output-type pprof --output-file profile.pb.gz --arguments-file arguments.json --open-in-browser
```

## Output Types

- **flamegraph**: Interactive SVG visualization, no additional dependencies required
- **pprof**: Google's profiling format, requires Go toolchain for visualization but provides more analysis tools

## Notes

- Only `main` entrypoint wrapped with `#[executable]` attribute is supported, you also have to have `[lib]` target in Scarb.toml so that a Sierra file is generated
- Arguments format is compatible with `scarb execute` but not with `scarb cairo-run`
- User and corelib as well as libfuncs are counted, providing the most detailed info
- Loops and recursive calls are collapsed to improve readability
- `--open-in-browser` opens SVG directly for flamegraphs, starts pprof web UI on port 8000 for pprof files
- `--no-build` flag to skip rebuilding the package

## Arguments Format

Arguments can be provided via file (--arguments-file) or command line (--arguments):
```json
["0x1234", "0x5678"]  // arguments.json example
```

Note that main executable wrapper has a single argument of type `Array<felt252>` containing inner arguments serialized with Cairo Serde.