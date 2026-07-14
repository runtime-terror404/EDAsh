# Changelog

All notable changes to edash will be documented in this file.

## Unreleased

### Added
- Self-contained install/update: `edash update`, `edash repair`, `install.sh`
- Catalog merge: `CatalogSource::Default` with base+user XDG overlay
- Hidden `__internal` subcommands for installer (paths, stage-catalog, self-test)
- `installation.yaml` metadata tracking
- `.github/workflows/release.yml` — tag-triggered CI (x86_64 + aarch64)
- `Resolver::load_from()` / `Resolver::load_default()` public API

### Changed
- Catalog resolution defaults to XDG paths (`~/.local/share/edash/catalog/base/`)
- `-c` / `EDASH_CATALOG_PATH` preserved for development
- Dev mode no longer falls back to `CARGO_MANIFEST_DIR`

### Fixed
- Double path join in `resolve_env()` breaking TUI tool table
- `tui/screens/catalog.rs` hardcoded `CARGO_MANIFEST_DIR` for PDK loading
- Removed dead `src/config.rs`

### Removed
- `CARGO_MANIFEST_DIR` fallback for catalog path (use `-c ./catalog` for dev)

---

## 0.1.0 (2026-07-14)

Initial release.

### CLI
- `install`, `list`, `remove`, `env`, `shell`, `doctor`, `search`, `why`, `outdated`, `clean`, `cache`, `export`, `pdk`
- Resolves environment names (digital, analog), tool names, and PDK names through a unified catalog
- `edash shell <env>` spawns subshell with PATH and PDK variables
- `eval "$(edash env <env>)"` exports without subshell
- `edash doctor` runs functional checks per tool

### TUI
- Dashboard with sidebar (envs + PDKs + downloads), search bar, and content area
- Keyboard-driven: j/k navigate, i/r install/remove, d doctor, E shell, / search, ? help, q quit
- Background threads for install/doctor with progress events
- Confirm overlay for destructive actions
- Viewport scrolling with cursor tracking

### Backends
- Micromamba (litex-hub, conda-forge, vlsida-eda channels) — one conda prefix per tool
- OSS CAD Suite (monolithic tarball from GitHub releases) — shared install for synthesis/FPGA/formal tools
- Bootstrapping: micromamba auto-offered on first use; ciel fail-fast with install command

### PDK management
- Per-PDK YAML configs with verified paths (sky130, gf180, ihp-sg13g2)
- Ciel integration for PDK fetch/enable
- `edash pdk` shows installed PDKs, env vars, and per-tool usage

### Catalog
- `tools.yaml` flat registry (single source of truth)
- `index.yaml` maps environments + PDK entries
- Environment files are name-lists referencing `tools.yaml`
- Adding a tool or PDK requires only YAML — zero code changes

### Lockfile
- System-global `edash.lock` (TOML) at `~/.local/share/edash/`
- Records exact versions, channels, backends, and PDK refs
- Idempotent installs — skips if package in lock and on disk

### Testing
- 95 integration tests across 11 files
