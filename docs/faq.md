# FAQ

## Why do some tools use micromamba and others OSS CAD Suite?

It depends on how upstream distributes them.

**Micromamba** handles tools available as conda packages — litex-hub, conda-forge, and vlsida-eda channels carry ASIC backend tools (openroad, magic, klayout, netgen) and analog tools (xschem, ngspice, xyce). Each tool gets its own isolated conda environment at `~/.local/share/edash/envs/_<name>/`. This keeps dependencies from conflicting.

**OSS CAD Suite** is a monolithic tarball (~700 MB) from YosysHQ that bundles synthesis (yosys), simulation (iverilog, verilator, gtkwave), FPGA (nextpnr, icestorm, prjtrellis, openfpgaloader), and formal tools (sby, boolector, z3) into one directory at `~/.local/share/edash/envs/oss-cad-suite/`. All tools in the tarball share a single install — that's how upstream ships them, so edash follows that pattern.

You don't need to know which backend a tool uses. `edash install digital` resolves everything automatically.

## Why isn't ciel auto-installed like micromamba?

Micromamba is a single static C++ binary (~25 MB, zero dependencies) — edash can fetch it to `~/.local/share/edash/bin/` with one `curl` call.

Ciel is a Python package (`pipx install ciel`). Auto-installing it would mean edash touches the user's Python environment — picking a pip, deciding between `--user`, a venv, or system-wide install. That's outside edash's scope. If ciel isn't found, edash prints the exact install command and exits:

```
ciel not found. Install: pipx install ciel
```

## What's the difference between `edash env` and `edash shell`?

`edash env` prints export statements:

```bash
eval "$(edash env digital)"   # activates in current shell, no subshell
```

`edash shell` spawns a new subshell:

```bash
edash shell digital           # new shell with tools on PATH, exit to go back
```

Use `env` when you want the tools in your current terminal. Use `shell` when you want an isolated session with a MOTD showing available tools.

## Why is the OSS CAD Suite install so slow?

It's a ~700 MB download from GitHub releases. The good news: it's downloaded once and cached at `~/.local/share/edash/cache/`. Subsequent installs (including for different tools from the same suite) skip the download if the cached tarball matches the latest release date. `edash cache` shows the current cache size.

## How do I add a tool that's not in the catalog?

Two ways:

**Edit the catalog directly** (contribute upstream):

1. Add the tool to [`catalog/tools.yaml`](catalog/tools.yaml)
2. Add its name to the environment file (`catalog/digital.yaml` or `catalog/analog.yaml`)
3. Send a PR

**User override** (keep it local):

Create `~/.config/edash/catalog/user/tools.yaml`:

```yaml
mytool: { backend: micromamba, channel: conda-forge, package: mytool }
```

User catalog entries override the base catalog. `edash update` never touches the user directory.

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for detailed instructions.

## Can I use edash offline?

Partially. Once tools are installed and the OSS CAD Suite tarball is cached, most operations work without internet: `list`, `env`, `shell`, `doctor`, `why`, `remove`.

What needs internet:

- Installing new tools (micromamba needs channel access, OSS CAD Suite checks for new releases)
- `edash update` and `edash outdated`
- PDK installs (ciel fetches from GitHub)

Full air-gapped support (pre-bundled tarballs) is tracked in [`docs/wip.md`](docs/wip.md).

## What happens if an install gets interrupted?

Tool installs are idempotent — re-run `edash install <name>` and it picks up where it left off. The lockfile isn't written until the install completes, so a partial install won't be recorded.

If `edash update` gets interrupted (power loss, Ctrl-C), run:

```bash
edash repair
```

It detects leftover staging files and either completes or rolls back the update. See [`docs/architecture.md`](docs/architecture.md) for the full crash-safety design.

## Why does the catalog have `gaw` when it has no CLI?

Gaw is a GUI waveform viewer — it doesn't have a `--version` flag or batch mode. It's in the analog environment because viewing waveforms is part of the analog design flow (open a `.raw` file from ngspice). `edash doctor` skips gaw since there's no CLI to check, but `edash shell analog` still puts it on your PATH.

## How do I use a different PDK variant?

Each PDK can have multiple variants. For example, sky130 has variants A and B with different library configurations; gf180mcu has A, B, C, and D.

The default variant is set in `catalog/pdks/<name>.yaml`. To use a different variant, override the PDK config in `~/.config/edash/catalog/user/pdks/`:

```bash
mkdir -p ~/.config/edash/catalog/user/pdks
cp ~/.local/share/edash/catalog/base/pdks/sky130.yaml \
   ~/.config/edash/catalog/user/pdks/sky130.yaml
# Edit the file — change variant: sky130A to sky130B
```

User configs override base configs, and `edash update` never touches them.

## How do I uninstall everything?

```bash
edash remove digital analog    # removes all tools (shared tools protected)
edash remove pdks              # removes all PDKs
rm -rf ~/.local/share/edash    # removes all edash data
rm ~/.local/bin/edash          # removes the binary
```

## Why do some tools fail to install on newer Ubuntu?

edash ships pre-computed explicit URL locks generated on Ubuntu 22.04 (glibc 2.35). Tools with a lock file install via direct URL fetch — the conda solver never runs, so distro-specific library versions don't affect resolution. This covers 7 of 8 micromamba tools (xschem, ngspice, xyce, openroad, magic, klayout, netgen).

If a tool has no lock file (user-added tools, or gaw which is known-broken on the `edash` channel), the fallback uses hermetic solver flags (`--override-channels --strict-channel-priority` + `CONDA_OVERRIDE_GLIBC=2.35`) that isolate it from the host's conda config and system library versions.
