# Noddy PowerToys Command Palette / PowerToys Run Extension (`noddy-ptrun`)

`noddy-ptrun` is a standalone, zero-dependency Rust helper that connects Windows launcher tools (PowerToys Command Palette, PowerToys Run, Flow Launcher, Raycast for Windows) to Noddy's [Local Authenticated API](../../docs/api.md).

## Building

```powershell
cargo build --release --manifest-path tools/powertoys-extension/Cargo.toml
```

## Prerequisites

1. Open **Noddy → Settings → Local automation API** and turn on **Enable local automation API**.
2. Noddy writes `%LOCALAPPDATA%\Noddy\api-token.json` containing the loopback port (`53320`) and a 256-bit Bearer token.

## Usage

### Querying Palette Items

```powershell
noddy-ptrun query ""
noddy-ptrun query "ring"
noddy-ptrun query "https://noddy.dev"
```

Outputs a JSON array of palette commands (`send-clipboard`, `ring`, `camera` `photo`/`scan`, `send-url`, `send-text`, plus recent timeline items).

### Invoking an Action

```powershell
noddy-ptrun invoke ring
noddy-ptrun invoke send-clipboard
noddy-ptrun invoke camera photo
noddy-ptrun invoke camera scan
noddy-ptrun invoke send-url "https://noddy.dev"
noddy-ptrun invoke send-text "Copied from PowerToys"
noddy-ptrun invoke send-file "C:\Users\Alice\Desktop\notes.pdf"
```
