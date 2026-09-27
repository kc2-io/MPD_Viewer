# Feature-packet execution rules

The repository-root `AGENTS.md` is authoritative for scope, safety, review,
coordination, verification, Git/PR delivery, and external-action permissions.
This file adds only the conventions for a task explicitly assigned from the
historical `docs/feature-plans/` packet. It does not activate every packet task
or proposed contract.

## Work unit

One worker receives one task packet, a verified base commit, a worktree, and an
explicit writable-file lease. Read that packet and the current code before
expanding scope. Return a small patch instead of opportunistic cleanup.

Source-file ownership is exclusive across worktrees. Experimental spikes may
touch overlapping code only when isolated and not merged; return findings and a
minimal reproduction. Shared Rust/controller/permission files and `ui/app.js`
remain coordinator-serialized.

Model names and effort levels in packet examples are historical suggestions.
Use available capabilities appropriate to task risk and cost, preserve current
tool settings, and report a meaningful substitution only when it affects the
result. Do not copy example configuration over active user/repository settings.

## Packet-specific boundaries

- Preserve the exact tagline `View fav channels in priority` and the identity,
  Twitch, authority, and evidence invariants in `docs/PRODUCT-CONTRACT.md`.
- No browser-engine replacement, Leptos rewrite, automatic chat, artificial
  engagement, ad blocking, unsupported Twitch APIs, cookie transfer, credential
  scraping, or fabricated viewer/rewards claims.
- The proposed grid, surface protocol, reset APIs, and authentication contracts
  in this packet are not implemented merely because the packet exists.
- A research-only task completes with bounded findings and evidence; it does not
  silently become an implementation, deployment, or account-administration task.

## Required packet handoff

Use the root contract's final handoff, adding these task fields when relevant:

```text
Task packet / verified base commit / worktree:
File lease and files changed:
Behavior implemented or research finding:
Checks run: exact commands and outcomes
Checks blocked or not run: reason and needed environment
Native runtime / wrapper version where relevant:
Known limitations and remaining decisions:
Security or migration implications:
Commit / PR and suggested dependency order:
```

Task completion means its acceptance criteria passed or its explicitly
research-only deliverable was produced. Writing code or a test is not evidence
that it ran successfully.
