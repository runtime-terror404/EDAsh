# Glossary

## EDA domain terms

**ASIC**: Application-Specific Integrated Circuit. A chip designed for a specific purpose (not a general-purpose processor). The digital environment targets the ASIC flow: RTL → synthesis → place & route → layout → verification.

**Analog design**: Circuit design dealing with continuous signals (voltages, currents) rather than digital 0s and 1s. edash's analog environment provides xschem (schematic capture), ngspice/xyce (simulation), magic/klayout (layout), and gaw (waveform viewing).

**DRC**: Design Rule Check. Verifies that a physical layout obeys the foundry's manufacturing rules (minimum spacing, wire width, etc.). Run inside magic or klayout.

**FPGA**: Field-Programmable Gate Array. A reconfigurable chip — you load a bitstream and it becomes your circuit. edash supports FPGA toolchains (nextpnr, icestorm, prjtrellis, openfpgaloader) via the OSS CAD Suite backend.

**gf180 / GF180MCU**: GlobalFoundries 180nm mixed-signal CMOS process. An open-source PDK available via ciel. Variants: gf180mcuA through gf180mcuD.

**ihp-sg13g2 / IHP SG13G2**: IHP's 130nm SiGe BiCMOS process. An open-source PDK available via ciel. Includes both CMOS and bipolar transistors — used for high-frequency/RF designs.

**LVS**: Layout vs. Schematic. Verifies that a physical layout matches the schematic it was drawn from. Run with netgen.

**Netlist**: A text file describing a circuit's components and how they're connected. SPICE netlists are the input to ngspice/xyce simulation; Verilog netlists are the output of synthesis.

**PDK**: Process Design Kit. A set of files from a foundry defining the rules and models for manufacturing on a specific process (transistor models, DRC rules, LVS rules, standard cell libraries, etc.). Open-source PDKs (sky130, gf180, ihp-sg13g2) are managed by edash via ciel.

**Place & route**: The step after synthesis in a digital flow — placing standard cells on the chip area and routing wires between them. openroad handles this in edash.

**RTL**: Register-Transfer Level. A hardware description (usually Verilog or VHDL) describing a digital circuit at the level of registers and the logic between them. The input to synthesis.

**Shuttle / MPW**: Multi-Project Wafer. A program (like Tiny Tapeout or Efabless chipIgnite) where multiple designers share a single wafer fabrication run to split costs. Open-source PDKs have made shuttle programs accessible to individuals.

**sky130 / SKY130**: SkyWater 130nm CMOS process. The most widely-used open-source PDK. Available via ciel with variants A and B.

**SPICE**: Simulation Program with Integrated Circuit Emphasis. The standard format for analog circuit simulation. ngspice and xyce both consume SPICE netlists. PDK configs provide SPICE model files for each process.

**Standard cells**: Pre-designed, pre-verified logic gates (AND, OR, flip-flops, etc.) that form the building blocks of a digital ASIC. Included in the PDK.

**Synthesis**: Converting RTL (Verilog) into a gate-level netlist using standard cells from the PDK. yosys handles this in edash.

**Tapeout**: The final step before manufacturing — submitting the completed layout (GDSII file) to the foundry. edash doesn't do tapeout directly, but the tools it installs produce the files you need.

---

## edash-specific terms

**Backend**: An implementation of the `Backend` trait that handles installing, verifying, and removing a category of tools. Current backends: micromamba (conda packages) and OSS CAD Suite (monolithic tarball). See [`docs/architecture.md`](docs/architecture.md) — Backends.

**Catalog**: A set of YAML files defining what edash can install (`tools.yaml` + environment files + PDK configs). The official catalog ships with the binary; user overrides go in `~/.config/edash/catalog/user/`. See [`docs/architecture.md`](docs/architecture.md) — Catalog.

**ciel**: A Python tool for managing open-source PDKs (formerly called Volare). edash shells out to `ciel ls-remote`, `ciel fetch`, and `ciel enable` to install PDKs. See [`docs/architecture.md`](docs/architecture.md) — PDK management.

**Environment**: A named group of tools (e.g. `digital`, `analog`). Defined as a YAML file listing tool names. Resolved at install time: each tool name is looked up in `tools.yaml` to find its backend and channel. See [`docs/architecture.md`](docs/architecture.md) — Catalog.

**Lockfile**: `~/.local/share/edash/edash.lock` — a TOML file recording exactly what is installed (package names, versions, channels, backends, PDK refs). The unit of reproducibility: same lock, same bits, any machine. Written by `edash install`, read by every other command. See [`docs/architecture.md`](docs/architecture.md) — Lockfile.

**micromamba**: A static C++ binary (~25 MB) that acts as a drop-in conda replacement. edash uses it to create per-tool conda environments at `~/.local/share/edash/envs/_<name>/`. Bootstrapped on first use — if micromamba isn't on PATH, edash offers to fetch it.

**OSS CAD Suite**: A monolithic tarball (~700 MB) from [YosysHQ](https://github.com/YosysHQ/oss-cad-suite-build) containing synthesis, simulation, FPGA, and formal tools as statically-linked binaries. edash downloads it once and caches it at `~/.local/share/edash/cache/`. All OSS CAD Suite tools share a single install at `~/.local/share/edash/envs/oss-cad-suite/`.

**PDK config**: A YAML file in `catalog/pdks/<name>.yaml` defining the PDK's variant name and relative paths to technology files (SPICE models, DRC rules, LVS setup, etc.). Used by `edash env` and `edash shell` to export per-PDK environment variables. See [`CONTRIBUTING.md`](CONTRIBUTING.md) — Adding a PDK.

**Prefix**: The on-disk directory where a tool is installed. For micromamba tools: `~/.local/share/edash/envs/_<toolname>/`. For OSS CAD Suite: `~/.local/share/edash/envs/oss-cad-suite/`. The lockfile records what's in each prefix.

**Resolver**: The component that turns a name (`digital`, `yosys`, `sky130`) into concrete install requests with backends and channels. Handles the base+user catalog merge. Defined in [`src/catalog/resolver.rs`](../src/catalog/resolver.rs).
