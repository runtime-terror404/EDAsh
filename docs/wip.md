# Work in progress / future

## Known bugs

### TUI download progress is fake

The download progress bars in the TUI use fixed increments (`saturating_add(3)` per tick) rather than actual byte counts from the backend. The `Progress` enum has a `Bytes { done: u64, total: u64 }` variant for real progress, but neither the micromamba nor oss-cad-suite backends emit it — they only send `Stage("...")` strings. The channel plumbing exists (`ProgressTx`), it just needs the backends to report real numbers.

**What needs to happen:**
- Micromamba: parse `micromamba install` output or use `--json` for download progress
- OSS CAD Suite: read curl's progress output (already downloading to a temp file)
- Both backends send `Progress::Bytes` events through the channel

---

## Incomplete features

### `edash export --format dockerfile`

Generates a Dockerfile but has a gap: the `COPY --from=edash` line assumes the edash binary exists in a previous build stage, but doesn't define that stage. A complete Dockerfile would either `curl` the binary from a release URL or copy it from the host.

**What needs to happen:**
- Add a `FROM` stage that downloads edash via `install.sh` or `curl`
- Or copy the current binary from the host (`COPY ./edash /usr/local/bin/edash`)
- Test with `docker build -f Dockerfile .`

### `edash export --format offline-bundle`

Declared but prints a stub message and exits. The idea is to create a self-contained tarball of tools + PDKs + lockfile that can be transferred to an air-gapped machine via USB — relevant for IP-controlled EDA labs that don't have internet access.

**What needs to happen:**
- Bundle all `envs/` directories (or a subset for a specific environment) into a tarball
- Include the lockfile and catalog
- Provide a companion `import` command (or flag) to restore from the bundle

### `manifest/schema.rs` — project pin file

The data types and file parser for `edash.yaml` exist in `src/manifest/schema.rs` (53 lines) but nothing calls them. The idea: put an `edash.yaml` in your project root, and edash auto-activates the right environment when you `cd` into the directory — same pattern as `.nvmrc` (Node) or `rust-toolchain.toml` (Rust).

What already works:
- `Manifest::from_file(path)` — parses YAML into a struct
- `Manifest::find_upwards(start_dir)` — walks up from cwd looking for `edash.yaml`
- Data model: environments list, PDK overrides, tool version overrides

```yaml
# Example ~/projects/my-asic/edash.yaml
environments: [digital]
pdk:
  sky130: { variant: sky130A }
overrides:
  yosys: ">=0.56"
```

**What needs to happen:**
- A shell hook (`eval "$(edash hook)"` in `.zshrc`/`.bashrc`) that wraps `cd` to check for `edash.yaml`
- Feed the manifest into the resolver so `edash install` respects manifest overrides
- The `overrides` field is parsed but not validated against the catalog (no version constraint solver exists)

---

## Not started (future)

### Additional backends

The original architecture spec listed three backends beyond micromamba and oss-cad-suite. The `Backend` trait is ready — any new backend implements `resolve()`, `install()`, `verify()`, `remove()` and slots into the existing catalog and actions layers.

- **Nix** (`nix-eda` channel): Higher ceiling for reproducibility than conda, but requires Nix installed. Would be a `NixBackend` struct implementing the `Backend` trait.
- **Distrobox** (podman/docker): Container-based installs that auto-share the host's X11/Wayland with the container — avoids the GUI-forwarding pain of raw Docker. Would shell out to `distrobox` CLI.
- **Source** (build from tarball): For tools not in any channel or tarball. The `ToolEntry` type already has `repo` and `requires` fields for this. Would fetch tarballs, run build scripts, and install to `envs/_<name>/`.

None of these are needed currently — all 25 tools in the catalog are covered by micromamba and oss-cad-suite. These are aspirational.

### PDK wrapper scripts

Currently, EDA tools don't auto-detect PDK files. After `edash shell digital`, tools are on PATH but you still pass PDK paths manually:

```bash
magic -rcfile $SKY130A_MAGIC_RCFILE
xschem --rcfile $SKY130A_XSCHEM_RCFILE
```

A wrapper script would wrap the real binary and inject those flags automatically, so typing `magic` in an activated shell finds the PDK without extra flags.

**What needs to happen:**
- Generate small shell scripts in `envs/_magic/bin/` that prepend the real binary with PDK flags
- Done per-environment on `edash env` / `edash shell`
- Needs to not break when multiple PDKs are installed (which one takes priority?)

### Release signing

The `install.sh` script downloads over HTTPS but doesn't cryptographically verify the binary. Release signing would let users verify authenticity:

```bash
curl -O https://github.com/.../edash-x86_64
minisign -Vm edash-x86_64 -P edash.pub
```

**What needs to happen:**
- Generate a minisign keypair, keep the secret key offline
- Sign each release binary in CI (or manually before publishing)
- Commit the public key to the repo
- `install.sh` fetches the `.minisig` alongside the binary and verifies before installing
- `edash update` does the same verification
