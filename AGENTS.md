# AGENTS.md

Shared guidance for AI coding agents in this repository.

Before changing this branch, read `CLAUDE.md` and `TRANSLATE_PLAN.md`.
`CLAUDE.md` remains the detailed project instruction file for historical
Claude Code compatibility; its scope, edit restrictions, and translation
rules apply equally to Codex and any other agent invoked by the scripts.

The translation driver scripts under `scripts/` can select the provider
with `--agent claude|codex` or `XLATE_AGENT=claude|codex`.  See
`scripts/README.md` for the full set of agent-selection environment
variables and logging paths.
