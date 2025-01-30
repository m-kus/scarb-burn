# Scarb Burn

Scarb extension for generating Cairo flame charts

## Installation

```bash
cargo install --git https://github.com/m-kus/scarb-burn scarb-burn
```

## Usage

```bash
scarb burn --arguments-file arguments.json --output-file flamegraph.svg --open-in-browser
```

## Notes

- Only `main(input: Array<felt252>)` entrypoint is supported
- Arguments format is compatible with `scarb excute` but not with `scarb cairo-run`
- User functions, corelib function, and libfuncs are displayed
- Loops and recursive calls are collapsed
