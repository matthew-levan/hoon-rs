# Hoon LSP

Language support for the [Hoon programming language](https://docs.urbit.org/hoon/why-hoon)
in VS Code (and VS Code–compatible editors such as VSCodium, Cursor, and Windsurf),
powered by the [`hoon-lsp`](https://github.com/matthew-levan/hoon-rs) language server.

## Features

- **Syntax highlighting** for `.hoon` files.
- **Parse diagnostics** — syntax errors are reported inline as you type.
- **Hover documentation** — hover a symbol to see its documentation.
- **Go-to-definition** — jump to a symbol's definition, including across files and
  through `/+`, `/-`, `/=` imports, preferring user-file definitions over the standard
  library when both exist.

The server is built on a Rust port of the
[Nockchain Hoon parser](https://github.com/nockchain/nockchain).

## Supported platforms

Prebuilt `hoon-lsp` binaries are bundled for:

- macOS — Apple Silicon (arm64) and Intel (x86_64)
- Linux — x86_64
- Windows — x86_64

On these platforms the extension works out of the box with no extra setup. On any other
platform, build `hoon-lsp` from source (see below) and put it on your `PATH`; the
extension falls back to a `hoon-lsp` binary found there.

## Requirements

None for the supported platforms above — the language server ships with the extension.

## Extension settings

| Setting | Default | Description |
| --- | --- | --- |
| `hoonLsp.debounceMs` | `75` | Milliseconds to wait after a file change before re-parsing. |
| `hoonLsp.workspaceRefreshMs` | `30000` | Milliseconds between workspace re-index cycles. |
| `hoonLsp.maxWorkspaceFiles` | `10000` | Maximum number of files to index in the workspace. |
| `hoonLsp.followSymlinks` | `true` | Whether to follow symlinks when indexing the workspace. |
| `hoonLsp.trace.server` | `off` | Trace LSP communication in the Output panel (`off`, `messages`, `verbose`). |

## Building from source

If your platform isn't covered by the bundled binaries:

```bash
git clone https://github.com/matthew-levan/hoon-rs
cd hoon-rs
cargo build --release -p hoon-lsp
# Put the binary on your PATH so the extension can find it:
export PATH="$PWD/target/release:$PATH"
```

You'll need the Rust toolchain specified in the repo's `rust-toolchain.toml`.

## Source & issues

This extension lives in the [`matthew-levan/hoon-rs`](https://github.com/matthew-levan/hoon-rs)
repository. Bug reports and contributions are welcome on the
[issue tracker](https://github.com/matthew-levan/hoon-rs/issues).

## License

MIT
