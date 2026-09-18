# MPD Viewer — Implementation planning packet

This packet plans the next feature milestone. It does not contain an updated app binary, modified application source, or a claim that any feature test passed.

Start with [PLAN.md](PLAN.md). The detailed contracts are in [CONTRACTS.md](CONTRACTS.md), inspected-source evidence in [SOURCE-REVIEW.md](SOURCE-REVIEW.md), and official references in [SOURCES.md](SOURCES.md).

## Use with a coding coordinator

1. Copy the planning documents, `tasks/`, and `task-manifest.json` into a documentation folder such as `docs/mpd-viewer-v0.2/` in the **current** working repository. Do not replace the repository with the reviewed old archive.
2. Merge [AGENT-RULES.md](AGENT-RULES.md) with existing project instructions. Review [codex-examples/](codex-examples/README.md); copy selected role TOML files into the repository's `.codex/agents/` and merge the example settings only after checking local model availability. Do not overwrite existing config or credentials.
3. Give the coordinator [ORCHESTRATOR-PROMPT.md](ORCHESTRATOR-PROMPT.md), including this packet's actual location. It starts at MV-000 and assigns [16 scoped task packets](tasks/README.md) through their dependency graph.

## What is covered

MPD Viewer branding; the exact tagline `View fav channels in priority`; per-viewer official chat; viewer website sign-in/Turbo diagnosis; a real single-window grid versus standalone viewers; session/surface identity; profile and popup safety; backward-compatible hosted wrapper delivery; native acceptance and release gates.

## Delivery contents

`task-manifest.json` records the model, effort, dependencies, allowed files, acceptance criteria, and checks for each task. `codex-examples/agents/` contains six inactive role configurations. `reports/packet-validation.json` records checks of this planning packet's structure only.

The source archive SHA and line-level evidence make assumptions reviewable. The deployed website and the user's native account session were not accessible for verification in this planning pass. The packet explicitly requires those observations before claiming that Turbo behavior is fixed or a platform supports the new presentation mode.
