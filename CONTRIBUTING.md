# Contributing

## Adding a tool to the catalog

Every installable tool lives in [`catalog/tools.yaml`](catalog/tools.yaml). There are two backends:

### Micromamba tools (conda packages)

For tools available in a conda channel, add an entry like this:

```yaml
# In catalog/tools.yaml
mynewtool: { backend: micromamba, channel: conda-forge, package: mynewtool }
```

- `backend`: always `micromamba`
- `channel`: the conda channel name (e.g. `conda-forge`, `litex-hub`, `vlsida-eda`)
- `package`: the exact package name in that channel

Find the right channel and package name with `micromamba search`:

```bash
micromamba search -c conda-forge mynewtool
```

### OSS CAD Suite tools (monolithic tarball)

Tools included in the [OSS CAD Suite](https://github.com/YosysHQ/oss-cad-suite-build) tarball don't have individual packages — they all share one install. Add just the backend:

```yaml
# In catalog/tools.yaml
mynewtool: { backend: oss-cad-suite }
```

No `channel` or `package` field — the tarball is self-contained.

### Adding the tool to an environment

After adding the entry to `tools.yaml`, add the tool name to the environment file. Environment files are simple name-lists — they only reference tool names, never redefine backend or channel:

```yaml
# In catalog/digital.yaml — add to the existing list
name: digital
tools:
  [
    openroad,
    klayout,
    magic,
    netgen,
    yosys,
    gtkwave,
    iverilog,
    verilator,
    nextpnr,
    icestorm,
    prjtrellis,
    openfpgaloader,
    sby,
    boolector,
    z3,
    mynewtool,          # ← add here
  ]
```

Or for the analog environment:

```yaml
# In catalog/analog.yaml
name: analog
tools: [xschem, ngspice, xyce, magic, klayout, netgen, gaw, mynewtool]
```

### Creating a new environment

To add an entirely new environment, you need two things:

1. Create the environment file (e.g. `catalog/custom.yaml`):

```yaml
name: custom
tools: [yosys, magic, netgen, gtkwave]
```

2. Register it in `catalog/index.yaml` under `environments:`:

```yaml
environments:
  digital: digital.yaml
  analog: analog.yaml
  custom: custom.yaml          # ← add this line
```

Now `edash install custom` works.

## Adding a PDK

PDKs have two pieces: an entry in `catalog/index.yaml` and a config file in `catalog/pdks/`.

### 1. Create the PDK config

Create `catalog/pdks/<name>.yaml`:

```yaml
name: mypdk
variant: mypdkA
paths:
  spice_dir: libs.tech/ngspice
  netgen_setup: libs.tech/netgen/mypdk_setup.tcl
  magic_rcfile: libs.tech/magic/mypdk.magicrc
  xschem_rcfile: libs.tech/xschem/xschemrc
  klayout_tech: libs.tech/klayout
```

- `name`: matches the filename stem (e.g. `mypdk` for `mypdk.yaml`)
- `variant`: the ciel/variant directory name on disk (e.g. `sky130A`, `gf180mcuD`)
- `paths`: relative paths from `~/.local/share/edash/pdks/<variant>/` to technology files. Supported keys are `spice_dir`, `netgen_setup`, `magic_rcfile`, `xschem_rcfile`, `klayout_tech`. Omit any that don't apply.

### 2. Register the PDK

Add an entry to `catalog/index.yaml` under `pdks:`:

```yaml
pdks:
  sky130: { manager: ciel, variant: sky130A }
  gf180: { manager: ciel, variant: gf180mcuD }
  ihp-sg13g2: { manager: ciel, variant: ihp-sg13g2 }
  mypdk: { manager: ciel, variant: mypdkA }       # ← add this line
```

- `manager`: `ciel` for PDKs fetched via ciel. Source-built PDKs are not yet supported.
- `variant`: must match the `variant` field in the PDK config file.

### 3. Verify

```bash
cargo run -- -c ./catalog pdk mypdk
```

Should show the PDK name, variant, install status, and environment variables.

## Overriding the catalog (users)

Users can override any catalog entry without editing the shipped files. Create matching files under `~/.config/edash/catalog/user/`:

```
~/.config/edash/catalog/user/
├── tools.yaml        # add new tools or override existing ones
├── index.yaml        # add new envs/PDKs or override existing ones
├── digital.yaml      # override the digital environment's tool list
└── pdks/
    └── sky130.yaml   # override sky130 paths
```

User entries take priority over the base catalog. `edash update` never touches the user directory.

## Development

```bash
# Build
cargo build

# Run tests (95 tests, 11 files)
cargo test

# Dev mode — point at the source tree catalog
cargo run -- -c ./catalog install digital

# Launch the TUI
cargo run -- -c ./catalog
```

## Before submitting a PR

- `cargo test` passes with no failures
- `cargo build --release` succeeds with no warnings
- New catalog entries are tested against the actual tool or PDK:
  ```bash
  cargo run -- -c ./catalog install <new-tool>
  cargo run -- -c ./catalog doctor <new-tool>
  ```
- If adding a PDK, verify the paths exist in the installed PDK directory

[`docs/wip.md`](docs/wip.md) tracks known bugs, incomplete features, and future plans — another place to find something to work on.
