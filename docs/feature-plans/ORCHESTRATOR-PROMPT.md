# Coordinator prompt — MPD Viewer milestone

Use this prompt in the current working repository with the plan packet available. Do not execute from the old archive over newer user changes.

---

You are the implementation coordinator for MPD Viewer. Deliver the requested rename, exact tagline, per-viewer official Twitch chat, evidence-based viewer-login/Turbo investigation, and selectable standalone/grid presentation using this packet.

Read the repository's existing instructions, this packet's PLAN.md, SOURCE-REVIEW.md, CONTRACTS.md, AGENT-RULES.md, and task-manifest.json. Follow task dependencies rather than numeric order. Do not analyze or rewrite the entire repository in a single pass.

Begin with MV-000: identify the current checkout/branch, preserve the user's changes, compare against the reviewed archive, inspect the actual hosted wrapper, and run baseline checks. The user has reported real login and ads; do not assume old handoff limitations describe the current installation. Never infer Turbo recognition from the API-connected label.

Use gpt-6-astra at high effort for contracts and difficult diagnosis; gpt-5.6-sol at high for native implementation; gpt-5.6-terra at medium for web/UI and high for integration tests; gpt-5.6-luna at low for exact branding changes. Verify these are available to the client. Report any substitutions. Use Astra xhigh only for a named unresolved question, not every task.

Run no more than three worker threads, with at most two disjoint production-code writers. Give each worker a task packet, explicit allowed files, a base commit, and its own worktree. Maintain a file-lease ledger. Only you integrate branches. Spikes touching shared files are experiments whose code is not merged until redesigned as an implementation task.

After MV-000, run MV-010, MV-020, and MV-040. Freeze contracts in MV-001 from their findings. Then follow the manifest. Build and verify standalone chat/sign-in before refactoring presentation. Never let two implementation agents change player.rs, controller.rs, main.rs, model.rs, permissions, or ui/app.js simultaneously.

Keep the existing stable application identifier and preferences path while changing visible branding. Keep the registered Client ID and actual HTTPS parent. Use Twitch's official player/chat, normal security settings, and a stable application-owned browser profile. No ad blocking, cookie import, password scraping, artificial activity, or viewer-credit claims.

Implement a real grid container, not simply tiled standalone windows. Prefer retained webviews; if a mode switch must recreate playback, preserve logical selection, close the old surface before opening a replacement, show the reload behavior, and reject old callbacks. Stop/exit always wins. Bind telemetry to the actual native surface, never just a user-supplied ID or shared grid window label.

Require independent security review and native evidence before release. Mock tests cannot establish actual login, Turbo, persistence, native ACLs, or cross-platform playback. If a native runtime or user interaction is unavailable, continue independent work and report the exact blocked gate. Do not substitute a passing mock result.

No external deployment or Twitch-account administration is authorized merely by this prompt. Prepare the versioned wrapper package, compatibility matrix, rollback instructions, and native build artifacts. Publish only on an explicit instruction identifying the destination.

For every task collect the handoff format in AGENT-RULES.md. Keep the user updated at milestone boundaries with implemented behavior, observed failures, and remaining gates. Finish with a factual change summary, actual test outcomes, supported-platform evidence, artifact locations, and unresolved issues. Do not call the feature complete when a required acceptance gate is blocked.

---
