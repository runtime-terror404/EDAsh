# edash

Reproducible EDA toolchain manager — rustup/pyenv semantics, for VLSI, FPGA, and analog design tools.

## Quickstart

```bash
# Install edash
curl -sSL https://raw.githubusercontent.com/runtime-terror404/edash/main/install.sh | sh

# Install a toolchain
edash install digital

# Check everything works
edash doctor digital

# Launch the dashboard
edash
```

## What it does

edash installs and manages EDA tools the way rustup manages Rust toolchains: one command, reproducible across machines, no system package manager involved. It wraps existing distributors instead of replacing them — micromamba for conda packages, OSS CAD Suite for FPGA/formal tools, and ciel for PDKs.

- **Two backends, one interface.** ASIC tools come from conda channels (litex-hub, conda-forge, vlsida-eda). Synthesis, simulation, FPGA, and formal tools come from the OSS CAD Suite tarball. You don't need to know which is which — `edash install digital` resolves everything.
- **Lockfile is the unit of reproducibility.** `edash.lock` records exact versions, channels, and backends. Same lock, same bits, any machine.
- **TUI dashboard.** `edash` with no arguments launches a keyboard-driven terminal UI — browse environments, install tools, run diagnostics.
- **PDK management.** `edash install sky130` fetches and enables the SKY130 PDK via ciel. `edash env` exports PDK path variables for magic, netgen, xschem, klayout, and ngspice.
- **Self-updating.** `edash update` fetches the latest release with crash-safe atomic swaps. `edash repair` recovers from interrupted updates.

See the [FAQ](docs/faq.md) for common questions or the [glossary](docs/glossary.md) for EDA terminology.

## Installation

### Linux (x86_64, aarch64)

```bash
curl -sSL https://raw.githubusercontent.com/runtime-terror404/edash/main/install.sh | sh
```

Installs to `~/.local/bin/edash`. The script detects your architecture, downloads the latest release, and sets up the catalog.

To install system-wide (CI, lab machines):

```bash
curl -sSL https://raw.githubusercontent.com/runtime-terror404/edash/main/install.sh | sh -s -- --system
```

### From source

```bash
git clone https://github.com/runtime-terror404/edash
cd edash
cargo build --release
./target/release/edash -c ./catalog install digital
```

The `-c ./catalog` flag points to the catalog in the source tree. Release binaries find the catalog at `~/.local/share/edash/catalog/base/` automatically.

### Dependencies

edash itself is a single static binary. At runtime it needs:

| Dependency | Required for | Auto-installed? |
|-----------|-------------|----------------|
| `micromamba` | Conda-based tools (openroad, magic, xschem, etc.) | Offered on first use |
| `ciel` | PDK installation | No — `pip install ciel` |
| `curl`, `tar` | OSS CAD Suite, self-updates | Expected on all Linux systems |

## Usage

```
$ edash --help
Reproducible EDA toolchain manager

Usage: edash [OPTIONS] [COMMAND]

Commands:
  install     Install environments, tools, or PDKs
  list        List installed packages and PDKs
  remove      Remove environments, tools, or PDKs
  env         Print shell exports (eval-able)
  shell       Spawn a subshell with tools on PATH
  doctor      Run functional checks on installed tools
  search      Search the catalog
  why         Show which environments pull in a tool
  outdated    Check for newer versions
  clean       Remove unreferenced installs
  cache       Show download cache usage
  export      Export to Dockerfile or GitHub Actions
  pdk         Show PDK configuration and usage
  update      Self-update to the latest release
  repair      Recover from interrupted updates
  help        Print this message or the help of the given subcommand(s)

Options:
  -c, --catalog-dir <CATALOG_DIR>  Override catalog path [env: EDASH_CATALOG_PATH]
  -h, --help                       Print help
  -V, --version                    Print version
```

### Environments

```bash
edash install digital    # ASIC backend + synthesis + FPGA + formal (18 tools)
edash install analog     # Analog design: xschem, ngspice, xyce, magic, klayout, netgen, gaw
```

### Individual tools

```bash
edash install yosys magic netgen
edash search open
edash why yosys          # shows which envs pull in yosys
```

### PDKs

```bash
edash install sky130
edash pdk                # list available and installed PDKs
edash pdk sky130         # show paths and per-tool usage
```

### Shell activation

```bash
eval "$(edash env digital)"   # export vars in current shell
edash shell digital           # spawn a subshell with MOTD
```

## Tools

### Digital environment

| Category | Tools | Backend |
|----------|-------|---------|
| Synthesis | yosys | OSS CAD Suite |
| Place & route | openroad | micromamba (litex-hub) |
| Layout | magic, klayout | micromamba |
| LVS | netgen | micromamba |
| Simulation | iverilog, verilator, gtkwave | OSS CAD Suite |
| FPGA | nextpnr, icestorm, prjtrellis, openfpgaloader | OSS CAD Suite |
| Formal | sby, boolector, z3 | OSS CAD Suite |

### Analog environment

| Category | Tools | Backend |
|----------|-------|---------|
| Schematic | xschem | micromamba (litex-hub) |
| Simulation | ngspice, xyce | micromamba |
| Layout | magic, klayout | micromamba |
| LVS | netgen | micromamba |
| Waveform | gaw | micromamba (edash) |

## Comparison

edash doesn't replace these tools — it wraps them and adds a reproducibility layer:

| vs | What edash adds |
|----|----------------|
| Raw `micromamba install` | Lockfile for reproducibility. One command installs entire environments. `doctor` runs functional checks, not just file-exists. |
| OSS CAD Suite tarball | Same lockfile + catalog layer. edash adds the ASIC and analog side under one CLI. |
| Nix / nix-eda | Different tradeoff. Nix's ceiling is higher (perfect reproducibility); edash's floor is lower (no Nix install required).
| Docker-based flows (IIC-OSIC-TOOLS) | No container overhead. Native GUI tools work without X11 forwarding. |

## Configuration

The catalog defines what can be installed. It's YAML — human-editable, no code changes needed to add a tool or PDK.

- **Official catalog**: `~/.local/share/edash/catalog/base/` (shipped with releases, updated by `edash update`)
- **User overrides**: `~/.config/edash/catalog/user/` (never touched by updates)

See [`docs/architecture.md`](docs/architecture.md) for the full catalog schema, lockfile format, backend architecture, and on-disk layout.

## Contributing

The easiest contribution is adding a tool to [`catalog/tools.yaml`](catalog/tools.yaml).
Check [`docs/wip.md`](docs/wip.md) for open issues and future plans.
See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Acknowledgments

edash wraps three upstream projects that do the heavy lifting:

- [litex-hub](https://github.com/litex-hub/conda-packages) and [conda-forge](https://conda-forge.org/) — conda packaging for open-source EDA tools
- [OSS CAD Suite](https://github.com/YosysHQ/oss-cad-suite-build) — monolithic distribution of synthesis, simulation, FPGA, and formal tools
- [ciel](https://github.com/efabless/ciel) (formerly Volare) — PDK version manager from Efabless/ChipFlow

TUI design inspired by [torlink](https://github.com/baairon/torlink). Toolchain-manager model inspired by [rustup](https://rustup.rs) and [mise](https://mise.jdx.dev).

## License

[MIT](LICENSE)
