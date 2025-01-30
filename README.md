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

## Limitations

- Only `main(input: Array<felt252>)` entrypoint is supported