# MPD Viewer — Implementation Agent Rules

Merge these project-specific rules with the repository's existing instructions. Do not overwrite its `AGENTS.md`, security policies, or Codex settings.

## Work unit

One agent receives one task packet, one base commit, one worktree, and an explicit writable-file lease. Read the packet and its referenced contracts first; inspect only the related paths before expanding. Return a small patch rather than opportunistic cleanup.

Use the assigned model and effort when available. Report substitutions. Do not claim that a model, browser, test suite, or operating system ran when it did not. Escalate a concrete unresolved decision to the coordinator, not the entire task to a vague rewrite.

A source file lease is exclusive even across worktrees if both changes will merge. Experimental spikes may touch overlapping code only when clearly isolated and not merged; deliver findings and minimal reproduction instead. Shared Rust/controller/permission files and `ui/app.js` are coordinator-serialized.

## Scope and safety

Preserve the existing Client ID, HTTPS parent hostname, settings identity, favorites, volume semantics, session-cap semantics, and user-pause behavior. The user-facing brand is MPD Viewer; the exact tagline is `View fav channels in priority`.

No browser-engine replacement, Leptos rewrite, auto-chat, artificial engagement, ad blocking, unsupported Twitch APIs, cookie import/export, credential scraping, browser-security disabling, or fabricated viewer/Turbo credit. Do not collect account credentials or unredacted authentication logs. Let the user perform Twitch password/MFA interaction in Twitch's UI.

Keep remote content separate from manager authority. A login popup has no custom native permissions. Source and response-header CSP must remain effective. Test least privilege with real native callers, not only a mocked JS bridge.

Do not publish website changes, change DNS, edit the Twitch registration, send messages, or create releases without explicit authorization for that external action. Produce local deployment assets and checklists when publication is not authorized.

## Tests and review

Run the smallest relevant checks while implementing, then the assigned integration checks. Do not weaken assertions or strip CSP just to turn a failure green. Mocks and test doubles are valid for logic tests, but label their evidence correctly. Native login, Turbo observations, persistent profiles, cross-platform window lifecycle, and native ACLs require native evidence.

Every implementation patch is reviewed independently. Security reviewer does not implement their own finding in the review pass: route it to the appropriate owning task, then re-review. Severe permission, data-loss, or duplicate-playback defects block release.

## Required handoff

Return this concise record:

```text
Task / base commit / worktree:
Model and effort actually used:
Files changed:
Behavior implemented:
Checks run: exact commands and outcomes
Checks blocked or not run: reason and needed environment
Native runtime / wrapper version where relevant:
Known limitations and remaining decisions:
Security or migration implications:
Patch/commit and suggested merge order:
```

Include links to bounded evidence files, not raw reasoning transcripts. Task completion means its acceptance criteria passed or its explicitly research-only finding was delivered. Writing code or a test is not the same as running it successfully.
