# Codex configuration examples

These files are inactive examples. They have not been executed through the user's Codex client.

After reviewing them, copy individual agent files into the repository's `.codex/agents/` directory. Merge `config.toml.example` with existing `.codex/config.toml`; do not replace existing settings, security controls, provider configuration, or credentials. The files use distinct `mpdv_` role names.

The task packet and file lease constrain each invocation; a model profile alone does not assign a task. Keep security review in a separate read-only agent run. The coordinator collects its report and writes any evidence file on its behalf. Workers must not rely on an implicit current task inherited from another conversation.

Model availability depends on client/account configuration. MV-000 verifies it. Report substitutions and adjust a role's configuration before launching it; do not silently fall back. The supplied TOML is syntax-checked only, not a claim that the local client loaded or ran it.

Sources: S10–S15 in `../SOURCES.md`.
