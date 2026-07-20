# Repository instructions

- Treat `README.md` and `README.zh-CN.md` as a synchronized pair.
- Add English and Simplified Chinese text for user-facing behavior in the same change.
- Keep inspection read-only. Never modify an input save in place.
- Do not implement save writes without dry run, explicit output, backup, isolated replacement, re-parsing, validation, and recovery guidance.
- Unknown save structures must fail closed; do not invent format assumptions.
- Keep machine-readable error codes and JSON fields stable across languages.
- Avoid new dependencies unless they provide substantial value and do not increase normal-user runtime requirements.
- Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` before completion.
- Do not describe planned or experimental functionality as implemented.

