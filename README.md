# Hoon-rs

Rust based tools for the [Hoon language](https://docs.urbit.org/hoon/why-hoon).

## Setup

You will need Rust nightly. Check `./rust-toolchain.toml` for details.
Nix users are recommended to use [Devenv](https;//devenv.sh). The included `devenv.nix` file will automatically build a developer environment for you.

## Packages

This is a Rust workspace containing several packages.

### Parser

The Hoon parser is a fork of the [Nockchain hoon parser](https://github.com/nockchain/nockchain/tree/bitemyapp/parser-parse-arm-comparison-next-sail-atom-markdown-testing-squashed).

To build run:
`cargo build -p hoon-parser`.

To test run:
`./target/debug/hoon-parser <filename>.hoon`

### LSP

A Language Server Protocol for Hoon. It depends on `hoon-parser`.

For details about the implementation status of the different LSP features please read ./LSP_STATUS.md written by OpenAI Codex 5.3.

To build run:
`cargo build -p hoon-lsp`.

To use you'll need to manually added to your IDE config.

#### VS Code-like editors
_Users please edit this_

#### Neovim:
_Users please edit this_

#### Helix:

Add the following under the hoon language block in your `languages.toml` file:

`language-servers = ["hoon-lsp"]`

Then add the following block somewhere in `languages.toml`:

```toml
[language-server.hoon-lsp]
command = "<absolute path to this repo>/target/debug/hoon-lsp"
```

### Formatter

A code formatter for Hoon. It depends on `hoon-parser`.

** NOTE** It is currently broken as it depends on a different fork of `hoon-parser` which can preserve the different wide/tall/irregular syntax of various runes. Coming soon™.

To build run:
`cargo build -p hoon-fmt`.

To test run:
`./target/debug/hoon-fmt <filename>.hoon`.

To use you'll need to manually added to your IDE config.

#### VS Code-like editors
_Users please edit this_

#### Neovim:
_Users please edit this_

#### Helix:

Add the following under the hoon language block in your `languages.toml` file:

`formatter = {command = "<absolute path to this repo>/target/debug/hoonfmt", args = ["-"]}`
