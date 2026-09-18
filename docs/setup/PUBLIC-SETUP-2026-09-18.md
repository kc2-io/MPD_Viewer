# Public repository setup — September 18, 2026

The owner explicitly changed `kc2-io/MPD_Viewer` to public. GitHub API readback
confirmed `isPrivate=false`. This record supersedes the earlier private-repository
visibility requirement and plan blocker; all signing and publication approvals
remain required. No agent changed the repository visibility or organization plan.

## Protections applied and verified

- `main` requires pull requests, linear history and resolved conversations.
  Administrators are subject to the rule; force pushes and deletion are disabled.
- Required, up-to-date checks are bound to the GitHub Actions application:
  `Source checks`, `Native (Windows-x64)`, `Native (macOS-arm64)`,
  `Native (macOS-x64)`, and `Native (Linux-x64)`.
- The active `Immutable release tags` ruleset prevents updates/deletion of `v*`
  tags, with no bypass actors.
- `release-windows`, `release-macos`, and `release-publish` require approval by
  `ipl31`. Administrator bypass is disabled. Self-review remains permitted for
  the documented solo-maintainer model.
- Each release environment allows only tag refs matching `v*`; branch deployments
  are not permitted.
- `RELEASES_ENABLED=false` and `STABLE_RELEASES_ENABLED=false` remain unchanged.

These settings were successfully accepted and read back after the public change.
Do not rerun the historical repository-creation helper against this existing repo.
Its new-repository path intentionally still defaults to private creation.

## Explicit release repository policy

`.github/release-policy.json` records the authorized repository and visibility:

```json
{
  "repository": "kc2-io/MPD_Viewer",
  "visibility": "public"
}
```

The tag gate checks the Actions repository identity and event visibility against
this committed policy. Publication checks `GH_REPO` plus GitHub's live `full_name`
and boolean `private` field before creating a draft and immediately before
publishing it. Missing, malformed, mismatched, or changed policy data stops the
operation. A changed visibility after upload leaves the draft unpublished.

This replaces the old private-only assertion; it does not remove visibility
validation. Forks and unexpected repository identities are rejected. The explicit
release flags, annotated/version-matched tags, main ancestry, pinned lock/toolchain/
Actions, native build matrix, signing verification, exact assets, checksum round
trip, and environment approval requirements remain intact.

## Signing and release work still required

Windows uses the existing Azure Artifact Signing service also used by BotOrNot
and mpd-bot. The references share a signing profile but use distinct application
IDs. MPD Viewer needs an authorized, narrowly scoped identity and exact OIDC trust:

```text
issuer: https://token.actions.githubusercontent.com
audience: api://AzureADTokenExchange
subject: repo:kc2-io/MPD_Viewer:environment:release-windows
```

The provider's Certificate Profile Signer role should be scoped to the existing
certificate profile. No new paid signing account/profile, broad organization
access, or changes to reference-repository trust are authorized. Environment
variables and provider trust must be verified separately; their presence alone
does not prove an actual signed artifact or working OIDC exchange.

The owner confirmed **no Apple signing credentials are available yet**. The
existing macOS Developer ID/notarization and complete release-asset gates remain
mandatory. Do not publish a Windows-only or unsigned substitute to bypass them.
Any different release-platform scope needs a separate explicit decision.

Keep the release flags off until prerequisites are verified. A specific reviewed
prerelease instruction is still required before creating/pushing a release tag
or publishing. The parent website is outside this setup and remains untouched.

## Validation scope

The policy change has regression tests for the approved public repository, wrong
repository/fork, changed/private/missing/invalid visibility, missing/malformed
policy, ordinary PR rejection, main-ancestry validation, and post-upload visibility
change preventing publication. These helper tests use mocks and are not evidence
of a real signing run or release upload.

The existing four-platform build/artifact result is recorded in
`CONTINUATION-2026-09-18.md`; the pull request for this policy change must also
satisfy all five required checks before merge. No application source, dependency,
branding, stored identity, or hosted HTML changes are included.
