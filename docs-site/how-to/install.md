# Install & Update

## Option A: Download a prebuilt binary (recommended)

1. Download the archive for your OS from GitHub Releases.
2. Extract it.
3. Put `rexos` (or `rexos.exe`) somewhere on your `PATH`.

Then:

```bash
rexos --help
rexos init
```

## Option B: Install from source (Cargo)

```bash
cargo install --path crates/rexos-cli --locked
rexos --help
```

## Update

- If you installed via Releases: download a newer archive and replace the binary.
- If you installed via Cargo: re-run `cargo install --path crates/rexos-cli --locked`.

