# MPD Viewer — start in Codex

Prepared: September 18, 2026. This is a source-and-context handoff, not a deployed application or completed GitHub setup.

## Open the project

Extract the project ZIP into a **new directory**. Open the inner `MPD_Viewer` folder in Codex as the local project. It contains `Cargo.toml`, `AGENTS.md`, source, plans, workflows, and a real `.git` directory. No `git init` or GitHub connection is required merely to open it. The packaged checkout has branch `main` and no remote.

In the current ChatGPT desktop app, select **Codex** from the top-left menu, then open the local folder/project and start a new chat. Interface labels may differ between app versions. The supported desktop workflow and separate Codex history are documented below. This package transfers working context; it does **not** import the complete ChatGPT conversation into Codex history.

Already have a newer checkout? Open that checkout instead. Compare the supplied baseline and import only the relevant handoff documents/changes. Never overwrite a newer checkout or copy this package's `.git` directory over another repository. The optional handoff patch is based on commit `603b8d4`; review it before applying.

Paste the contents of **[CODEX-START-PROMPT.md](CODEX-START-PROMPT.md)** into the first Codex chat. Start in the local checkout for repository/bootstrap work. Use separate worktrees for independent feature tasks later.

## Authentication on your machine

A ChatGPT/Codex sign-in and a GitHub sign-in are separate. The repeated plugin checks in the previous conversation did not establish usable repository access. Do not assume they establish the current state on this machine either.

If GitHub CLI is installed, use your local terminal:

```powershell
gh auth status --hostname github.com
```

If not authenticated, complete browser-based authorization yourself:

```powershell
gh auth login --hostname github.com --git-protocol https --web
```

Keep passwords, tokens, two-factor codes, certificates and signing passwords out of prompts, source files and logs. Do not print `gh auth token`. Let the CLI use normal secure credential storage. Do not use insecure storage or disable the sandbox to bypass a permission error. Review the locally requested permissions, including workflow-file write access if needed.

After authorization Codex should verify the identity and access, for example:

```powershell
gh api user --jq .login
gh repo view kc2-io/BotOrNot --json nameWithOwner,isPrivate,viewerPermission
gh repo view kc2-io/mpd-bot --json nameWithOwner,isPrivate,viewerPermission
```

Account sign-in alone does not prove permission to create a repository under `kc2-io`, access organization resources, configure protections, or use a signing service. Those are separate checks. A failed repository lookup may mean lack of access rather than nonexistence.

## Immediate objective

Finish the user's existing request: create/configure **private `kc2-io/MPD_Viewer`**, commit/push the source, inspect the actual BotOrNot and mpd-bot conventions, and complete build, signing and tagged-release automation.

The user already authorized that repository setup. Preserve privacy; do not require the user to restate known requirements. Inspect current remote state before creating anything. If the repository exists, inspect it and reconcile without overwriting or force-pushing. Do not change the two reference repositories; their files and settings are comparison inputs only.

The prepared setup is **not verified parity** with either reference repository. Read the source before executing bootstrap helpers. Remote resources, billing changes, credential scopes, and actual release publication each need their applicable checks/approvals. Do not create new paid signing resources, broaden organization-wide access, or expose source publicly.

## What the handoff includes

| Area | State at the prepared baseline |
|---|---|
| POC | Rust scheduling/controller, Tauri desktop shell, simple HTML/JS management UI, demo mode, persistence, Twitch adapter and standalone player code. Native integration remains unverified in this handoff. |
| Branding | Latest setup commit changed visible branding to **MPD Viewer** and **View fav channels in priority**. Recheck upgrade behavior; do not repeat a global rename. |
| Identity | Internal crate remains `mpd-tabber`; bundle identity remains `com.modpackdad.mpdtabber.poc`. Preserve stored preferences and viewer-profile identity. |
| Twitch application | Public client ID: `ha94kk20cfu1tp74pgg8isgi88cpo7`. This is not a secret or evidence of account authorization. |
| Parent site | `https://parent.mpdviewer.com/`; the user reports it works. Imported HTML is supplied source, not a verified live download. |
| Hosted source | `web/parent.mpdviewer.com/index.html` is an intact provenance snapshot with a DOM grid. It is **not protocol-compatible** with the bundled native adapter without changes. |
| Plans | Original architecture, 16 scoped feature tasks, agent profiles/examples, security/contracts guidance, and the later hosted-source addendum are included. |
| Repository automation | CI/bootstrap/tagged-release workflows and helper scripts are prepared. They are not validated by a successful real Actions run. |
| Builds/signing | No genuine native build, signed executable, notarized DMG, installer or published release is provided. |
| GitHub | No remote creation/push was performed by the previous handoffs. Verify actual current remote state rather than treating that historical fact as current absence. |

## Critical application mismatch

The supplied hosted HTML reads **query parameters**, fractional volume, and `postMessage` commands; the native POC uses **fragment parameters**, different audio calls and session-specific native reports. The hosted code also defaults to `pauseInactive=true`. Its existing DOM grid is a starting point, not completed grid integration.

Read `docs/HOSTED-SOURCE-PLAN-ADDENDUM.md` **before** the older feature coordinator prompt. Advance protocol alignment before chat/grid work and compare one webview containing a DOM grid with native child-webview presentation. Do not replace the site's single HTML file with the old three-file wrapper by assumption.

Monitoring OAuth is not viewer website sign-in and does not prove Turbo recognition. Legitimate website-session sharing and constrained login popups require native tests; no ad-blocking or cookie importing is planned. Preserve manual pause, normal autoplay rules and accurate playback reporting.

## Reading map and work order

1. `AGENTS.md` and `CODEX-START-PROMPT.md`: operating scope and first task.
2. `docs/setup/GITHUB-SETUP.md`, `SIGNING.md`, `RELEASING.md`, `VERIFICATION.md`: setup and actual outstanding gates.
3. Inspect `scripts/bootstrap-github.py`, `scripts/pin-actions.py`, `.github/workflows/` before invoking writes. Confirm existing reference workflows and signing settings locally.
4. Resolve a real `Cargo.lock`, toolchain and action pins. Run native builds/tests on appropriate hosts; fix failures before enabling signing/publication.
5. Complete scoped repository settings and signing trust using existing authorized resources where available. Verify results and private visibility through actual remote responses.
6. Then implement feature tasks using `docs/feature-plans/` and the hosted-source addendum. Keep infrastructure and feature commits separate.

Keep `RELEASES_ENABLED` and `STABLE_RELEASES_ENABLED` false until their recorded gates pass. Do not push a tag or publish as a test of whether the pipeline works without a specific reviewed prerelease instruction. Never silently publish unsigned output when signing fails. Do not deploy or change the parent website merely because the repository is being set up.

## Verification expectations

Historical setup reports record 74 passing offline tests. That does not establish native compilation, actual IPC permissions, live Twitch, authentication, Turbo, signing or GitHub Actions results. Rerun relevant tests and record fresh commands/results. `HANDOFF-VERIFICATION.md` in this package records the narrower checks performed for this transfer.

The package deliberately does not activate `.codex/config.toml`, install plugins, set approval modes, force model names, or change global Codex settings. Agent configurations under `docs/feature-plans/codex-examples/` are examples. Check models and reasoning controls available in the current client before using them.

Use one coordinator, at most three workers, and at most two production-code writers with disjoint file ownership. Keep each worker bounded to a module/task, use separate branches/worktrees, and use an independent reviewer for signing/authentication/lifecycle changes.

## Documentation used for the transfer

Checked September 18, 2026:

- OpenAI: [ChatGPT Work and Codex](https://help.openai.com/en/articles/20001275/) — separate Codex history and desktop entry point.
- OpenAI: [Desktop app](https://developers.openai.com/codex/app/) — open a local folder/project and choose Codex.
- OpenAI: [AGENTS.md instructions](https://developers.openai.com/codex/guides/agents-md/) — repository instruction discovery.
- GitHub: [CLI browser authentication](https://cli.github.com/manual/gh_auth_login) — local browser flow; account permissions still need verification.
