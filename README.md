# Scarb Burn

Scarb extension for generating Cairo flame charts.  

This is a complementary tool to [cairo-profiler](https://github.com/software-mansion/cairo-profiler), particularly useful if you have a program that consumes a large arguments file.  
Also it does not require any additional dependencies (like pprof and golang), just Rust.

## Installation

You need to have Rust installed on your machine.

```bash
cargo install --git https://github.com/m-kus/scarb-burn scarb-burn
```

## Usage

Run in your project directory.

```bash
scarb burn --arguments-file arguments.json --output-file flamegraph.svg --open-in-browser
```

## Notes

- Only `main` entrypoint wrapped with `#[executable]` attribute is supported
- Arguments format is compatible with `scarb execute` but not with `scarb cairo-run`
- User and corelib as well as libfuncs are counted, providing the most detailed info
- Loops and recursive calls are collapsed to improve readability
