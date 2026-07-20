# PalMerge

[English](README.md) | [简体中文](README.zh-CN.md)

PalMerge is a safety-first, local and open-source toolkit for understanding and eventually migrating or merging Palworld save data. The repository is at an early foundation stage. It does **not** currently merge, migrate, repair, or write save files.

The project direction and acceptance criteria are tracked in [Project Goal #1](https://github.com/g4d3Y3/PalMerge/issues/1).

## Current capabilities

- Read-only discovery of `Level.sav`, `LevelMeta.sav`, `LocalData.sav`, `WorldOption.sav`, and direct `.sav` files under `Players/`.
- Streaming SHA-256 fingerprints without external runtime dependencies.
- Conservative GVAS magic detection. Unknown inputs remain `unknown`; they are never guessed.
- Human-readable English and Simplified Chinese output.
- Stable, language-independent JSON inspection output.
- Deterministic ordering and explicit resource limits during discovery.

## Not implemented yet

- Full Palworld container decompression and GVAS property parsing.
- Domain models, entity indexes, dependency graphs, and referential validation.
- Migration, merge, repair, GUID rewriting, transactional writing, backup, rollback, or GUI workflows.

Do not use the current release as a save merger. Inspection is intentionally read-only.

## Use a prebuilt binary

Normal users should download the package for their operating system from [Releases](https://github.com/g4d3Y3/PalMerge/releases), extract it, and run `palmerge`. Rust, Cargo, Python, compilers, package managers, and network access are not required at runtime.

Prebuilt packages will be attached to tagged releases for Windows x86-64, Linux x86-64, macOS Apple Silicon, and macOS Intel. Until the first tagged release exists, contributors can build from source as described below.

## Inspect a save

```console
palmerge inspect /path/to/Level.sav
palmerge inspect /path/to/world-directory --lang zh-CN
palmerge inspect /path/to/world-directory --format json
```

The command reads and hashes files but never modifies them. JSON uses `schema_version: 1`, stable error codes, and untranslated machine fields.

## Build from source

Source builds are for contributors and advanced users only. Install Rust 1.77 or newer, then run:

```console
cargo build --release --locked
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The executable is written to `target/release/palmerge` (`palmerge.exe` on Windows).

## Safety and privacy

- Inspection is read-only.
- Unknown formats are reported rather than guessed.
- Save data stays on the local computer; core functionality performs no uploads or telemetry.
- Real private saves must not be committed as test fixtures.
- Future write support will require dry run, explicit output, backup, isolated writes, re-parsing, validation, and recovery guidance before it is described as production-ready.

## Workspace

- `palmerge-core`: stable errors, localization, fingerprints, and shared primitives.
- `palmerge-parser`: bounded world discovery and conservative format probing.
- `palmerge-cli`: scriptable text and JSON inspection interface.

The current crates intentionally use only the Rust standard library to keep builds portable and normal-user packages self-contained.

## Contributing

Keep changes small and reviewable. Add tests, run formatting and Clippy, preserve stable machine fields, update both README files, and include English and Simplified Chinese text for user-facing behavior. Never document roadmap items as completed features.

## License

MIT

