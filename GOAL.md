# PalMerge Project Goal

## Mission

PalMerge is an open-source toolkit for safely understanding, validating, migrating, merging, and managing Palworld save data.

The project must prioritize data safety, correctness, transparency, maintainability, localization, and ease of use. It should help ordinary users complete save-related operations with the lowest reasonable deployment and usage cost, while also providing reusable foundations for developers and future applications.

PalMerge must not merely edit binary data. It should understand the entities, references, dependencies, and structural constraints contained within Palworld saves, then perform operations in a deterministic, auditable, and recoverable way.

---

## Primary Goal

Build a safe, reliable, dependency-aware, multilingual, and easy-to-use Palworld save management platform that ordinary users can run with minimal setup and that developers can extend through stable Rust libraries and clearly separated modules.

The finished product should allow users to inspect, validate, migrate, merge, compare, repair, and manage Palworld saves without requiring them to understand save-file internals or install unnecessary development environments.

---

## Primary Objectives

1. Parse Palworld save files into explicit, structured, and testable domain models.

2. Identify relationships between players, pals, inventories, containers, guilds, bases, dynamic items, map objects, and other referenced entities.

3. Build a deterministic dependency graph that can explain which entities belong to or depend on a selected player, guild, base, or world object.

4. Validate save integrity before and after every operation.

5. Generate a complete dry-run plan before any write operation.

6. Support safe migration and merging while preserving referential integrity.

7. Perform all write operations transactionally, with backups, isolated output, verification, and rollback support.

8. Provide reusable Rust libraries for parsers, validators, dependency graphs, migration planning, writing, and verification.

9. Provide a user-friendly CLI and, eventually, a desktop graphical interface.

10. Minimize the deployment cost, environment requirements, learning cost, and operational complexity for ordinary users.

11. Provide complete Simplified Chinese support in documentation, command output, error messages, interactive interfaces, and the final user-facing product wherever practical.

---

## User Experience Goal

An ordinary user should be able to use PalMerge without installing Python, Rust, Cargo, compilers, package managers, or third-party save-editing tools.

The preferred user experience is:

```text
Download the package for the current operating system
→ Extract it
→ Launch the program
→ Select the source and destination saves
→ Review the dry-run result
→ Confirm the operation
→ Receive a verified output save and a readable report
```

PalMerge should reduce deployment and usage costs as much as reasonably possible.

This includes:

* providing prebuilt binaries for supported operating systems;
* avoiding unnecessary runtime dependencies;
* preferring a single executable or self-contained application package;
* avoiding mandatory configuration files for basic use;
* providing sensible defaults;
* detecting common save locations where possible;
* offering actionable error messages;
* providing both CLI and graphical workflows when practical;
* avoiding steps that require users to manually edit binary files, JSON structures, GUID values, or environment variables;
* packaging required static resources with the application;
* supporting offline use for core save-management functions;
* keeping installation, updating, backup, migration, and rollback procedures simple.

Source builds may require Rust and Cargo, but those requirements must apply only to contributors and advanced users, not to normal end users.

---

## Language and Localization Requirements

PalMerge must treat Simplified Chinese as a first-class supported language rather than an optional translated summary.

The finished product should support both English and Simplified Chinese wherever practical.

This applies to:

* `README.md`;
* `README.zh-CN.md`;
* installation documentation;
* user guides;
* CLI help;
* CLI output;
* warnings;
* validation reports;
* error messages;
* release notes;
* desktop interface text;
* confirmation dialogs;
* backup and rollback instructions;
* user-facing configuration;
* generated reports.

### README Requirements

* Maintain both `README.md` and `README.zh-CN.md`.
* `README.md` is the canonical English document.
* `README.zh-CN.md` must be a complete Simplified Chinese version, not a shortened summary.
* Both README files must remain structurally and semantically synchronized.
* Add language-switch links near the top of both files.
* Any change to installation instructions, commands, project status, supported features, architecture, safety warnings, or user workflow must update both README files in the same change.
* Do not document unfinished functionality as available.
* Clearly distinguish between:

  * prebuilt binary usage for ordinary users;
  * source-build requirements for contributors;
  * currently implemented features;
  * experimental features;
  * planned features.

### Product Localization Requirements

* Do not hard-code user-facing English strings throughout the codebase.
* Centralize translatable messages through an appropriate localization layer.
* English and Simplified Chinese should use stable translation keys.
* Missing translations should fall back safely and visibly.
* Error identifiers and machine-readable fields should remain stable across languages.
* Human-readable explanations may be localized, but machine-readable output must remain predictable.
* Tests should verify important English and Simplified Chinese messages.
* New user-facing features must include both English and Simplified Chinese text in the same change whenever practical.

---

## Design Principles

### 1. Safety Before Convenience

Save integrity is more important than speed, feature count, or implementation convenience.

No feature should be merged if it creates an unreasonable risk of silent corruption.

### 2. Read-Only by Default

Inspection, discovery, parsing, reporting, and validation must remain read-only.

Write behavior must require an explicit command or confirmation.

### 3. Never Modify the Original Save Directly

The original source save must remain untouched by default.

Write operations should produce a separate output directory or file.

### 4. Backup Before Write

Every write operation must create or verify a usable backup before modifying generated output.

Backups must be easy to identify and restore.

### 5. Dry Run Before Execution

Any migration, merge, repair, deletion, or GUID rewrite must support a dry-run mode.

The dry-run report should explain:

* selected source entities;
* discovered dependencies;
* planned additions;
* planned removals;
* planned GUID changes;
* conflicts;
* unresolved references;
* warnings;
* expected output location.

### 6. Dependency-Aware Operations

The project must treat saves as entity graphs rather than independent files.

Operations must follow explicit references and ownership relationships instead of relying on blind binary replacement.

### 7. Deterministic Behavior

Given the same inputs, configuration, and application version, PalMerge should produce the same plan, output, ordering, identifiers, and reports whenever possible.

### 8. Explainable Decisions

Every automatic decision should be traceable.

The user should be able to understand why an entity was selected, excluded, renamed, rewritten, or rejected.

### 9. Fail Closed

Unknown formats, unsupported versions, missing dependencies, ambiguous ownership, invalid references, and structural inconsistencies must block unsafe writes.

The software must not guess silently.

### 10. Reversible Operations

Every write operation must be recoverable through backups, isolated output, transaction records, or rollback mechanisms.

### 11. Minimal Dependencies

Use external dependencies only when they provide clear, significant value.

Avoid adding dependencies for functionality that can be implemented safely and clearly with the standard library or existing project components.

### 12. Low User Cost

Every feature should be evaluated partly by its effect on installation complexity, operational steps, user learning requirements, hardware requirements, and failure recovery.

### 13. Stable Machine Interfaces

Machine-readable output should use stable schemas and identifiers.

Human-readable output may be localized, but scripts should not need to parse translated sentences.

### 14. Privacy by Default

Save files may contain identifiers and private world information.

Do not upload, transmit, or collect save contents without explicit user action and clear disclosure.

Core functionality should work locally and offline.

---

## Architecture Goal

The codebase should evolve toward the following pipeline:

```text
Discovery
→ File Fingerprinting
→ Container and Compression Detection
→ Save Parsing
→ Domain Model
→ Entity Index
→ Dependency Graph
→ Validation
→ Conflict Detection
→ Migration or Merge Planning
→ Dry Run
→ Transaction Construction
→ Backup
→ Isolated Write
→ Re-parse
→ Semantic Verification
→ User Report
```

Each layer should have a clear responsibility and communicate through explicit types.

Recommended module boundaries include:

```text
palmerge-core
palmerge-parser
palmerge-graph
palmerge-validator
palmerge-planner
palmerge-writer
palmerge-report
palmerge-i18n
palmerge-cli
palmerge-gui
```

Modules should not bypass domain interfaces to manipulate raw save data directly unless that module is explicitly responsible for parsing or serialization.

---

## Domain Modeling Goal

Palworld save contents should be represented through explicit domain types wherever practical.

Examples include:

* player;
* player profile;
* player inventory;
* item container;
* equipment container;
* party pal;
* palbox entry;
* pal entity;
* guild;
* guild membership;
* base;
* base worker;
* dynamic item;
* map object;
* world object;
* spawn point;
* ownership relationship;
* entity reference;
* save version;
* migration conflict;
* validation issue.

Raw GUIDs, strings, and untyped maps should not be passed across major architectural boundaries when a stronger domain type is practical.

---

## Validation Goal

Validation must exist at multiple levels.

### File-Level Validation

Verify:

* required files exist;
* files are readable;
* files are not unexpectedly empty;
* file signatures are recognized;
* compression and container formats are supported;
* file sizes and hashes are recorded.

### Structural Validation

Verify:

* required properties exist;
* property types match expectations;
* identifiers have valid formats;
* collections are internally consistent;
* duplicate records are detected;
* unsupported format changes are reported.

### Referential Validation

Verify:

* referenced entities exist;
* containers resolve correctly;
* player-owned pals resolve correctly;
* guild memberships resolve correctly;
* base ownership resolves correctly;
* GUID rewrites are complete;
* no dangling references remain.

### Semantic Validation

Verify:

* entity ownership remains valid;
* selected entities form a consistent dependency closure;
* migrated inventories reference migrated containers;
* guild and base relationships remain coherent;
* newly written saves can be parsed again;
* the resulting world remains consistent with known Palworld rules.

---

## Write Safety Requirements

Any component that writes save data must:

1. refuse to modify the original input by default;
2. require an explicit output path;
3. create or verify a backup;
4. validate available disk space;
5. write through a temporary location;
6. avoid partial output becoming the final output;
7. flush and close files before replacement;
8. re-open and re-parse generated files;
9. run structural and semantic validation;
10. report exactly which files changed;
11. record before-and-after fingerprints;
12. provide recovery instructions;
13. stop immediately when verification fails.

A successful write means the generated output has been re-parsed and validated. Merely completing file output is not sufficient.

---

## CLI Requirements

The CLI should remain scriptable, predictable, and friendly to ordinary users.

It should provide:

* clear subcommands;
* useful `--help` output;
* English and Simplified Chinese display modes;
* human-readable output;
* stable JSON output;
* non-zero exit codes on errors;
* explicit dry-run support;
* explicit output paths;
* quiet and verbose modes;
* clear warnings before write operations;
* no interactive requirement when used in automation;
* optional interactive guidance for new users.

Machine-readable output must not depend on the selected user-interface language.

Prefer stable fields such as:

```json
{
  "code": "missing_entity_reference",
  "severity": "error",
  "entity_id": "...",
  "message": "Localized human-readable text"
}
```

---

## GUI Goal

The future graphical interface should make safe workflows accessible to non-technical users.

It should provide:

* English and Simplified Chinese interfaces;
* automatic save-location discovery where possible;
* drag-and-drop or file-picker selection;
* clear source and destination distinction;
* visual dependency summaries;
* conflict explanations;
* dry-run previews;
* explicit backup confirmation;
* progress visibility;
* cancellation where safe;
* validation results;
* rollback guidance;
* easy access to logs and reports.

The GUI must use the same core libraries, planners, validators, and writers as the CLI. It must not implement a separate save-manipulation engine.

---

## Distribution Goal

PalMerge should minimize installation and deployment work for users.

The project should provide prebuilt release packages for supported platforms, prioritizing:

* Windows x86-64;
* Linux x86-64;
* macOS Apple Silicon;
* macOS Intel when practical.

Release packages should include:

* the executable;
* English README;
* Simplified Chinese README;
* license;
* essential usage instructions;
* version information;
* checksums.

Where practical, provide:

* portable Windows ZIP packages;
* compressed Linux and macOS archives;
* package-manager installation;
* signed binaries;
* reproducible builds;
* automatic update guidance;
* release notes in English and Simplified Chinese.

The core CLI should not require network access after installation.

---

## Dependency Policy

Before introducing a new dependency, consider:

1. whether the feature can be implemented safely with the standard library;
2. whether an existing dependency already provides the required capability;
3. whether the dependency is actively maintained;
4. whether it increases executable size significantly;
5. whether it introduces native system requirements;
6. whether it complicates cross-compilation;
7. whether it affects offline use;
8. whether it creates security or licensing concerns;
9. whether it increases installation complexity for users;
10. whether a smaller optional feature flag is appropriate.

Avoid dependencies that require users to install external runtimes, shared libraries, language environments, database servers, web browsers, or background services for basic use.

Optional integrations should remain optional and feature-gated where practical.

---

## Quality Requirements

All new code should:

* compile without warnings;
* pass formatting checks;
* pass Clippy with warnings treated as errors;
* include tests whenever practical;
* preserve deterministic behavior;
* avoid global mutable state;
* avoid duplicated business logic;
* use explicit error types;
* provide useful error context;
* avoid unnecessary unsafe code;
* avoid unchecked indexing and unbounded allocation for untrusted inputs;
* place limits on recursion, collection sizes, and input sizes where appropriate;
* document public APIs;
* keep modules focused;
* preserve clear architectural boundaries;
* avoid breaking stable interfaces without justification.

Security-sensitive parsing and writing code should receive stronger review and testing than ordinary presentation code.

---

## Testing Requirements

Testing should include:

* unit tests;
* integration tests;
* parser fixture tests;
* malformed-input tests;
* regression tests;
* dependency-graph tests;
* cycle tests;
* deterministic-output tests;
* localization tests;
* dry-run snapshot tests;
* write-and-reparse tests;
* rollback tests;
* cross-platform path tests;
* large-input and resource-limit tests where practical.

Real save files must not be committed unless they are clearly licensed, minimized, synthetic, or explicitly sanitized.

Private user saves must never be added to public fixtures without informed permission.

---

## Documentation Requirements

Documentation is part of the product and must be updated alongside code.

Required documentation should cover:

* project status;
* supported game versions;
* supported operating systems;
* installation;
* portable usage;
* source builds;
* CLI commands;
* GUI workflows;
* backup and rollback;
* dry-run behavior;
* known limitations;
* save compatibility;
* security considerations;
* architecture;
* contribution rules;
* release procedures.

When behavior changes, update related documentation in the same pull request.

Examples and commands must reflect actual implemented behavior.

Do not present roadmap items as completed features.

---

## Development Workflow Requirements

For every meaningful change:

1. identify the affected architectural layer;
2. avoid unrelated refactoring;
3. add or update tests;
4. update both English and Simplified Chinese user-facing documentation;
5. run formatting;
6. run linting;
7. run the full relevant test suite;
8. verify machine-readable output compatibility;
9. check whether deployment or usage complexity increased;
10. summarize risks and limitations in the pull request.

When a change increases environment requirements or adds user setup steps, justify why the additional cost is necessary and consider a lower-cost alternative first.

---

## Codex Instructions

When implementing work in this repository, Codex should:

* inspect the existing architecture before editing;
* follow the project goal and safety rules;
* prefer small, reviewable changes;
* preserve backward compatibility where practical;
* avoid inventing unsupported save-format assumptions;
* clearly mark uncertain format behavior;
* add tests for new behavior;
* update `README.md` and `README.zh-CN.md` together;
* include Simplified Chinese text for new user-facing functionality;
* avoid adding unnecessary dependencies;
* avoid requiring new runtimes or services for ordinary users;
* prefer prebuilt, portable, offline-capable user workflows;
* never claim unfinished functionality is complete;
* keep inspection features read-only;
* require dry-run, backup, isolated output, and verification for writes;
* fail safely when save structures are unknown;
* use stable error codes for machine consumers;
* keep translated human messages separate from machine-readable fields;
* leave the repository in a compiling, formatted, tested state.

If the requested implementation conflicts with save safety, determinism, localization, privacy, or low user deployment cost, Codex should explain the conflict and implement the safest practical alternative.

---

## Non-Goals

PalMerge is not intended to:

* perform blind binary search-and-replace;
* silently guess unknown save structures;
* modify original saves without explicit authorization;
* require Python or other language runtimes for normal use;
* depend on unofficial editors at runtime;
* require internet access for core functionality;
* upload user saves automatically;
* prioritize quick hacks over long-term correctness;
* build separate logic for CLI and GUI;
* hide unresolved conflicts;
* report success without post-write verification;
* support every historical game version through unsafe heuristics;
* sacrifice data integrity for convenience or speed.

---

## Current Development Priority

Until complete parsing and validation are available, development should prioritize:

1. reliable world discovery;
2. save fingerprinting;
3. container and compression detection;
4. GVAS parsing;
5. explicit domain models;
6. Palworld entity indexing;
7. dependency resolution;
8. structural and referential validation;
9. deterministic dry-run planning;
10. transactional writing;
11. post-write verification;
12. portable releases;
13. complete English and Simplified Chinese user experience;
14. desktop GUI.

Write-enabled merging should not be treated as production-ready until dependency resolution, validation, backup, isolated output, and post-write verification are all implemented.

---

## Definition of Done

A user-facing feature is complete only when:

* its implementation is functional;
* its behavior is tested;
* errors are handled safely;
* it does not introduce unnecessary environment requirements;
* ordinary-user installation remains simple;
* English and Simplified Chinese interfaces are provided where applicable;
* `README.md` and `README.zh-CN.md` are updated when relevant;
* machine-readable output remains stable;
* implemented and unsupported behavior are clearly distinguished;
* security and privacy implications are considered;
* the feature works through the shared core architecture;
* CI passes.

A save-writing feature is complete only when it additionally supports:

* dry run;
* conflict reporting;
* backup;
* isolated output;
* transactional replacement;
* re-parsing;
* structural validation;
* referential validation;
* semantic verification;
* recovery instructions.

---

## Long-Term Vision

PalMerge should become the foundational open-source infrastructure for understanding and safely manipulating Palworld save data.

It should provide:

* a robust save parser;
* stable domain models;
* a dependency graph engine;
* a validation framework;
* a save diff engine;
* a migration and merge planner;
* a transactional writer;
* backup and rollback tools;
* a CLI;
* a desktop GUI;
* reusable Rust crates;
* multilingual user-facing documentation and interfaces.

The project should remain safe, local-first, transparent, portable, dependency-conscious, and approachable to both English-speaking and Chinese-speaking users.

PalMerge succeeds when an ordinary user can safely complete a complex save operation with minimal setup, understand what the program will do, obtain a verified result, and recover easily if anything goes wrong.
