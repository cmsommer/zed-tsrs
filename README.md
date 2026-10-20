# typescript-rs-ls: tsrs Language Server for Zed

This extension integrates [tsrs](https://github.com/oxc-project/tsrs) — a Rust port of TypeScript 7's native compiler and language server — into the Zed editor.

`tsrs` aims for TypeScript 7 conformance with lower memory use and faster type checking. It speaks the same language-server protocol as `tsgo` (`tsrs --lsp --stdio`).

## Installation

1. Open Zed's Extensions page (or install from a local path while developing).
2. Install the `TypeScript RS Language Server` extension (`typescript-rs-ls`).

## Configuration

### Basic Setup

Enable `typescript-rs-ls` in your Zed settings:

```jsonc
{
    "languages": {
        "TypeScript": {
            "language_servers": [
                "typescript-rs-ls",
                "!vtsls",
                "!typescript-language-server",
                "...",
            ],
        },
        "TSX": {
            "language_servers": [
                "typescript-rs-ls",
                "!vtsls",
                "!typescript-language-server",
                "...",
            ],
        },
    },
}
```

You can also use `typescript-rs-ls` alongside another TypeScript server (for example `vtsls`). Zed will use `typescript-rs-ls` for features it supports and fall back to the next language server for unsupported features:

```jsonc
{
    "languages": {
        "TypeScript": {
            "language_servers": [
                "typescript-rs-ls",
                "vtsls",
                "!typescript-language-server",
                "...",
            ],
        },
        "TSX": {
            "language_servers": [
                "typescript-rs-ls",
                "vtsls",
                "!typescript-language-server",
                "...",
            ],
        },
    },
}
```

Do not enable both this extension's server and the official `typescript-ls` (tsgo) extension on the same language unless you intentionally want two servers.

### Specifying a Package Version

By default, the extension installs and uses the latest version of the [`tsrs`](https://www.npmjs.com/package/tsrs) npm package. To pin a specific version:

```json
{
    "lsp": {
        "typescript-rs-ls": {
            "settings": {
                "package_version": "0.18.0-ts7.1.0-dev.20260929"
            }
        }
    }
}
```

### Platform Support

The extension installs the `tsrs` npm package, which pulls a matching optional `@tsrs/<platform>` binary. As of `tsrs` 0.18.0, published platforms include:

- `darwin-arm64`
- `linux-x64-gnu` / `linux-x64-musl`
- `linux-arm64-gnu` / `linux-arm64-musl`

Windows and `darwin-x64` binaries are not published yet. On unsupported platforms the extension will fail to locate a native binary after install.

### Inlay Hints

The extension sets defaults so `typescript-rs-ls` can send inlay hints when Zed has them enabled:

```json
{
  "inlay_hints": {
    "enabled": true
  }
}
```

Override defaults under `lsp.typescript-rs-ls.settings.typescript` (and `javascript` for JS):

```json
{
  "lsp": {
    "typescript-rs-ls": {
      "settings": {
        "typescript": {
          "inlayHints": {
            "parameterNames": {
              "enabled": "none"
            },
            "parameterTypes": {
              "enabled": false
            },
            "variableTypes": {
              "enabled": false
            },
            "propertyDeclarationTypes": {
              "enabled": false
            },
            "functionLikeReturnTypes": {
              "enabled": false
            },
            "enumMemberValues": {
              "enabled": false
            }
          }
        }
      }
    }
  }
}
```

### Code Lens

References and implementations code lenses are enabled by default. Turn on Zed's code lens setting:

```json
{
  "code_lens": "on"
}
```

Override under `lsp.typescript-rs-ls.settings.typescript` / `javascript` as needed.

## Local Development

From this repository:

1. In Zed, open the command palette and run **zed: install dev extension**.
2. Select this directory.
3. Configure `typescript-rs-ls` in your settings as above.

## Related

- [tsrs](https://github.com/oxc-project/tsrs) — Rust TypeScript 7 port
- [zed-extensions/tsgo](https://github.com/zed-extensions/tsgo) — official Zed extension for TypeScript's Go-based native server
