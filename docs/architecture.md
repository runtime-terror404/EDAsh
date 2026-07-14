# edash — architecture

_Reproducible EDA toolchain manager — rustup/pyenv semantics for VLSI, FPGA, and analog design tools._

## Design principles

- **Never touch the system package manager.** Tool versions come from conda channels (via micromamba), monolithic tarballs (oss-cad-suite), or PDK managers (ciel). No `apt`/`dnf`/`pacman`.
- **Lockfile is the unit of reproducibility.** Same lock, same bits, any machine. `edash.lock` records exact versions and backends.
- **Backend is an implementation detail behind a trait.** Micromamba, oss-cad-suite, and future backends share the same interface. No tool-specific branching outside `src/backend/`.
- **Catalog is data, not code.** Tools, environments, and PDKs are defined in YAML files — adding a tool or PDK requires zero Rust changes.
- **No telemetry, no phone-home** (except GitHub API for oss-cad-suite release checks and `edash update`).

## Repository layout

```
edash/
├── src/
│   ├── main.rs                    # Binary entry: TUI or CLI dispatch
│   ├── lib.rs                     # Clap CLI parser, subcommand dispatch, module root
│   ├── paths.rs                   # XDG path resolution (data, config, cache, catalog dirs)
│   ├── actions.rs                 # Shared install/remove orchestrator (CLI + TUI both call this)
│   ├── installation.rs            # InstallationMetadata — read/write installation.yaml
│   ├── catalog/
│   │   ├── mod.rs                 # CatalogSource enum (Path vs Default/base+user merge)
│   │   ├── index.rs               # Data types: CatalogIndex, ToolRegistry, BackendKind, etc.
│   │   ├── resolver.rs            # Name → PackageRequest resolution, search, listing
│   │   └── source.rs              # Dev-mode catalog path helper (CARGO_MANIFEST_DIR)
│   ├── backend/
│   │   ├── mod.rs                 # Backend trait + Progress/ResolvedPackage types
│   │   ├── micromamba.rs          # Conda-based backend (litex-hub, conda-forge, vlsida-eda)
│   │   └── oss_cad_suite.rs       # Monolithic tarball backend (synthesis, FPGA, formal)
│   ├── lockfile/
│   │   ├── mod.rs                 # Module root
│   │   ├── schema.rs              # Lockfile, LockedPackage, LockedPdk (TOML)
│   │   └── writer.rs              # Serialize/deserialize edash.lock
│   ├── manifest/
│   │   ├── mod.rs                 # Module root
│   │   └── schema.rs              # edash.yaml project manifest (WIP, not wired in)
│   ├── pdk/
│   │   ├── mod.rs                 # PdkRequest type
│   │   ├── ciel.rs                # PDK install via ciel (ls-remote → fetch → enable)
│   │   └── config.rs              # Per-PDK YAML configs, env var resolution
│   ├── doctor/
│   │   ├── mod.rs                 # Module root
│   │   └── checks.rs              # Functional micro-benchmarks per tool
│   ├── cli/
│   │   ├── mod.rs                 # Module declarations
│   │   ├── install.rs             # edash install <name...>
│   │   ├── list.rs                # edash list
│   │   ├── remove.rs              # edash remove <name...>
│   │   ├── env.rs                 # edash env <name> (eval-able exports)
│   │   ├── shell.rs               # edash shell <name> (subshell with MOTD)
│   │   ├── doctor.rs              # edash doctor <name>
│   │   ├── search.rs              # edash search <query>
│   │   ├── why.rs                 # edash why <tool>
│   │   ├── outdated.rs            # edash outdated
│   │   ├── clean.rs               # edash clean + edash cache
│   │   ├── export.rs              # edash export --format dockerfile|github-actions
│   │   ├── pdk.rs                 # edash pdk [name]
│   │   ├── update.rs              # edash update (self-update from GitHub releases)
│   │   ├── repair.rs              # edash repair (recover interrupted updates)
│   │   └── installer.rs           # __internal subcommands (gated, hidden from help)
│   └── tui/
│       ├── mod.rs                 # App, event loop, rendering, action dispatch
│       ├── widgets.rs             # Color constants, status spans, spinners
│       ├── screens/
│       │   ├── mod.rs             # Module root
│       │   └── catalog.rs         # CatalogScreen: sidebar, tool table, PDK table, downloads, doctor
│       └── overlays/
│           ├── mod.rs             # Module root
│           ├── help.rs            # Help overlay (? key)
│           └── confirm.rs         # Confirm overlay (destructive actions)
├── catalog/                       # Community-editable registry
│   ├── index.yaml                 # Environment list + PDK entries
│   ├── tools.yaml                 # Flat tool registry (single source of truth)
│   ├── digital.yaml               # ASIC backend + synthesis + FPGA + formal tools
│   ├── analog.yaml                # Analog design tools
│   └── pdks/
│       ├── sky130.yaml            # SKY130 PDK paths
│       ├── gf180.yaml             # GF180MCU PDK paths
│       └── ihp-sg13g2.yaml        # IHP SG13G2 PDK paths
├── tests/                         # Integration tests (95 tests, 11 files)
├── docs/
│   └── architecture.md            # This file
├── .github/workflows/
│   └── release.yml                # Tag-triggered release build (x86_64 + aarch64)
├── install.sh                     # Bootstrap: curl | sh first-install
├── Cargo.toml
├── CLAUDE.md                      # Session context and rules
└── PLAN.md                        # Build sequence + TUI spec
```

## Core concepts

### Catalog

The catalog is a set of YAML files that define _what_ can be installed. It lives in two places depending on context:

- **Dev mode** (`-c ./catalog` or `EDASH_CATALOG_PATH`): a single directory, typically the source tree.
- **Release mode** (default): `~/.local/share/edash/catalog/base/` (official, shipped with the binary) merged with `~/.config/edash/catalog/user/` (user overrides). User keys in `tools.yaml` and `index.yaml` override base; user environment files replace base ones.

Three YAML file types make up the catalog:

| File | Purpose |
|------|---------|
| `index.yaml` | Maps environment names → env YAML files. Lists available PDKs with manager and variant. |
| `tools.yaml` | Flat registry: every installable tool, its backend, and optional channel/package. Single source of truth. |
| `<env>.yaml` | Named list of tool names (e.g. `digital.yaml` lists 14 tools). No duplication — tool metadata lives in `tools.yaml`. |
| `pdks/<name>.yaml` | Per-PDK config: variant name and relative paths for spice, magic, netgen, xschem, klayout. |

### CatalogSource

`CatalogSource` is an enum that abstracts over where the catalog comes from:

- **`CatalogSource::Path(PathBuf)`** — explicit directory (dev mode). All files read from that single directory.
- **`CatalogSource::Default`** — XDG merged catalog. Reads from `catalog_base_dir()` first, then overlays `catalog_user_dir()` on top. User entries win.

Every CLI command and the TUI receive a `CatalogSource` and pass it through to the resolver and PDK config loader.

### Resolver

`Resolver` loads the catalog and resolves names into concrete package requests:

- **Environment name** (`digital`, `analog`): expands to the list of tools defined in the env YAML file, each looked up in `tools.yaml`.
- **Tool name** (`yosys`, `magic`): resolves directly from `tools.yaml` into a `PackageRequest` with backend, channel, and package fields.
- **PDK name** (`sky130`, `gf180`): resolves into a `PdkRequest` with manager and variant.

Resolution is polymorphic — `install digital` and `install yosys` hit the same resolver. The resolver also powers `search`, `why`, and environment listing.

When loading from `CatalogSource::Default`, the resolver merges base and user catalogs at construction time — user `tools.yaml` keys override base keys, user env files replace base ones. Environment file resolution at runtime checks the user dir first, then falls back to base.

### Backends

Backends implement the `Backend` trait and handle the actual install/verify/remove mechanics:

```
pub trait Backend {
    fn name(&self) -> &'static str;
    fn resolve(&self, req: &PackageRequest) -> Result<ResolvedPackage>;
    fn install(&self, pkg: &ResolvedPackage, progress: ProgressTx) -> Result<()>;
    fn verify(&self, pkg: &ResolvedPackage) -> Result<bool>;
    fn remove(&self, pkg: &ResolvedPackage) -> Result<()>;
}
```

Two backends are implemented:

| Backend | What it installs | Mechanism |
|---------|-----------------|-----------|
| **Micromamba** | ASIC tools (openroad, magic, klayout, netgen), analog tools (xschem, ngspice, xyce, gaw) | `micromamba create -p envs/_<tool> <channel>::<package>` — one conda prefix per tool |
| **OSS CAD Suite** | Synthesis (yosys), simulation (iverilog, verilator, gtkwave), FPGA (nextpnr, icestorm, prjtrellis, openfpgaloader), formal (sby, boolector, z3) | Single ~700 MB tarball from GitHub releases, extracted to `envs/oss-cad-suite/`. All tools share one installation. Cached by release date. |

`actions.rs` calls backends — no backend-specific logic exists outside `src/backend/`.

### Lockfile

`~/.local/share/edash/edash.lock` is a TOML file that records exactly what is installed:

```toml
version = 1
generated = "2026-07-14T12:00:00Z"

[[package]]
name = "yosys"
version = "oss-cad-suite"
backend = "oss-cad-suite"
sha256 = ""

[[package]]
name = "magic"
version = "8.3.465_0_g5477395"
channel = "litex-hub"
backend = "micromamba"
sha256 = ""

[pdk.sky130]
variant = "sky130A"
manager = "ciel"
ref = "d658698bd8bcf4e05fc7b5991a701247ba0d744c"
sha256 = ""
```

The lockfile is the system's source of truth for idempotency — `install` checks it first and skips if a package is already recorded and present on disk. `remove` prunes entries. All other commands read it.

### Actions

`src/actions.rs` is the shared orchestrator between CLI and TUI. Every install and remove operation — whether triggered by `edash install digital` or pressing `i` in the TUI — goes through the same functions:

- `install_tool()` — idempotent, dispatches to the right backend, updates the lockfile
- `install_pdk()` — delegates to `ciel`, records in lockfile
- `remove_tool()` / `remove_pdk()` / `remove_all_pdks()` — clean up disk and lockfile
- `remove_env()` — removes an environment's tools, protecting shared tools used by other envs
- `is_tool_installed()` — checks both lockfile and disk

### PDK management

PDKs are managed through two subsystems:

**Installation** (`pdk/ciel.rs`): Wraps the `ciel` CLI tool. `install_pdk("sky130")` calls `ciel ls-remote`, `ciel fetch`, and `ciel enable`. PDKs are installed to `~/.local/share/edash/pdks/` with symlinks at the root (e.g. `pdks/sky130A → ciel/sky130/versions/<ref>/sky130A`).

**Configuration** (`pdk/config.rs`): Per-PDK YAML files in `catalog/pdks/` define paths to technology files. `resolve_pdk_vars()` generates environment variables like `SKY130A_SPICE_DIR`, `GF180MCUD_MAGIC_RCFILE`, etc. These are exported by `edash env` and `edash shell`.

## CLI surface

| Command | Behavior |
|---------|----------|
| `install <name...>` | Resolves each arg (env, tool, or PDK), installs, updates lockfile |
| `list` | Installed packages + PDKs with versions, channels, and disk status |
| `remove <name...>` | Env-level removal (protects shared tools) or individual tool/PDK removal |
| `env <name>` | Prints `export` statements for PATH, PDK_ROOT, and per-PDK vars |
| `shell <name>` | Spawns subshell with environment activated, prints MOTD |
| `doctor <name>` | Functional checks: runs each tool against a minimal test |
| `search <query>` | Case-insensitive search across envs, tools, and PDKs |
| `why <tool>` | Shows which environments pull in a given tool |
| `outdated` | Compares locked versions against latest available |
| `clean [--dry-run]` | Removes unreferenced install directories |
| `cache` | Reports download cache size |
| `export --format <fmt>` | Generates Dockerfile or GitHub Actions workflow |
| `pdk [name]` | Lists PDKs or shows per-PDK config and usage |
| `update` | Self-update: fetches latest GitHub release, atomic catalog+binary swap |
| `repair` | Recovers from interrupted updates (stale `.old`/`.new` dirs) |

## Self-update mechanism

`edash update` implements crash-safe self-updates:

1. Fetches latest release tag from GitHub API
2. Short-circuits if already at that version (checks `installation.yaml`)
3. Downloads binary + catalog tarball to `downloads_dir()`
4. Sanity-checks the new binary (`--version`)
5. Extracts catalog and stages it via the hidden `__internal stage-catalog` subcommand
6. Atomic two-rename swaps: `catalog/base → base.old`, `base.new → base`; then `edash → edash.old`, new → edash
7. Runs `__internal self-test` against the live binary
8. On success: cleans up `.old` dirs, writes `installation.yaml`
9. On failure: restores `edash.old`, keeps new catalog (already validated)

`edash repair` handles interrupted updates by checking for leftover `.old`/`.new` dirs and `edash.old` binaries, completing or rolling back as appropriate. It can be run manually or is suggested at startup if stale files are detected.

Hidden subcommands (`__internal paths`, `__internal stage-catalog`, `__internal self-test`) are gated behind `EDASH_INSTALLER=1` and excluded from `--help` — they exist only for `install.sh` and `edash update` to use.

## TUI

The TUI launches when `edash` is invoked with no arguments and stdout is a TTY. It's built with `ratatui` + `crossterm`:

- **Three-panel layout**: sidebar (environments + PDKs + downloads), search bar, content area (tool table / PDK table / doctor results / download queue)
- **Keyboard-driven**: `j/k`/`↑↓` navigate, `←→`/`tab` switch panes, `i` installs, `r` removes, `d` runs doctor, `E` opens a shell, `/` searches, `?` shows help, `q` quits
- **Background threads**: install and doctor operations run in spawned threads, reporting progress through MPSC channels. The main event loop drains these channels each tick
- **Non-TTY fallback**: when stdout is not a terminal, the same operations run as linear CLI output

The TUI receives a `CatalogSource` and passes it through to the resolver and spawn functions, same as the CLI path. There is no code path divergence between CLI and TUI for catalog loading, installation, or removal.

## On-disk layout

```
~/.local/share/edash/          ($XDG_DATA_HOME/edash)
├── edash.lock                  # System-wide lockfile (TOML)
├── installation.yaml           # Install metadata (method, version, timestamp)
├── catalog/
│   └── base/                   # Official catalog (shipped, updated by `edash update`)
│       ├── index.yaml
│       ├── tools.yaml
│       ├── digital.yaml
│       ├── analog.yaml
│       └── pdks/*.yaml
├── envs/                       # Tool installations
│   ├── _yosys/                 # Per-tool micromamba prefixes
│   ├── _magic/
│   ├── _openroad/
│   ├── ...
│   └── oss-cad-suite/          # Shared monolithic tarball install
├── pdks/                       # PDK data (ciel-managed)
│   ├── sky130A → ciel/sky130/versions/<ref>/sky130A
│   ├── gf180mcuD → ...
│   ├── ihp-sg13g2 → ...
│   └── ciel/                   # Ciel internal store
├── cache/                      # Download cache (oss-cad-suite tarballs)
├── downloads/                  # Staging area for updates
├── bin/                        # Bootstrapped binaries (micromamba) + edash itself
└── logs/                       # Subprocess output logs

~/.config/edash/               ($XDG_CONFIG_HOME/edash)
└── catalog/
    └── user/                   # User catalog overrides (never touched by updates)
        ├── tools.yaml          # Additional/override tool definitions
        ├── index.yaml          # Additional/override envs or PDKs
        └── pdks/*.yaml         # Additional/override PDK configs
```

## External dependencies at runtime

| Dependency | Purpose | Required | Bootstrap |
|-----------|---------|----------|-----------|
| `micromamba` | Conda-based tool installs | For micromamba tools | Auto-offered on first use |
| `ciel` | PDK fetch/enable | For PDK installs | Fail-fast with install instructions |
| `curl` | oss-cad-suite download, `edash update` | For oss-cad-suite + updates | Expected on all Linux systems |
| `tar` | oss-cad-suite extraction, catalog staging | For oss-cad-suite + updates | Expected on all Linux systems |

## Tech stack

| Component | Choice | Why |
|-----------|--------|-----|
| Language | Rust (edition 2021) | Static binary, zero runtime deps |
| CLI | `clap` 4 (derive) | Subcommand parsing, env var binding |
| TUI | `ratatui` 0.29 + `crossterm` 0.28 | Terminal UI with cross-terminal support |
| Async | `tokio` 1 | Subprocess management, channel-based progress |
| Serialization | `serde_yaml` (catalog), `toml` (lockfile) | Human-readable, hand-editable formats |
| HTTP | `curl` subprocess (API, downloads) | oss-cad-suite, edash update |
| Path resolution | `dirs` 6 | XDG base directories |
| Hashing | `sha2` + `hex` | Content verification |

## Testing

95 integration tests across 11 files in `tests/`. Key areas covered:

- **Resolver**: environment resolution, tool resolution, PDK resolution, search, `which_envs`, listing, multi-backend, `load_from`
- **Catalog types**: `BackendKind`, `ToolEntry`, `EnvironmentDef`, `PdkEntry`, `CatalogIndex` deserialization and roundtrips
- **Paths**: all 14 path functions, XDG correctness, absolute paths
- **CatalogSource**: `Path` variant, `list_pdk_names`, `read_pdk_config`, clone
- **Lockfile**: write/read roundtrip, package lookup, PDK serialization, empty sections
- **Installation metadata**: read/write roundtrip, overwrite, serde
- **PDK config**: all three PDK configs, env var resolution, path verification
- **Installer**: `__internal` subcommand gating, help visibility, version output
- **Actions**: `is_tool_installed` sanity checks

Tests use `tempfile` for isolation and the real `catalog/` directory from the repo for data-driven tests. The installer tests spawn the actual `edash` binary for end-to-end CLI verification.
