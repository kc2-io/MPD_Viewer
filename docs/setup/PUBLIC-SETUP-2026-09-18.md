# Public repository setup — September 18, 2026

> Current alpha scope: [ALPHA-RELEASE-PLAN.md](ALPHA-RELEASE-PLAN.md). The owner approved signed Windows-only alpha distribution; historical full-platform requirements below still apply to other release stages.

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

Windows reuses the existing Azure Artifact Signing account `MPD-Artifacts` and
certificate profile `mpd-bot-public`, also used by the read-only reference repos.
The Azure login succeeded. The live profile is active and its publisher matches
the independently verified reference executable.

The dedicated single-tenant application `mpd-viewer-github-signing` and its
service principal were created without passwords, certificate credentials, or
requested Graph/API permissions. The application client ID is
`39ca9c57-3bf4-47ea-b5ae-69fe86f8ad8b`; all seven nonsecret Windows environment variables,
including `AZURE_CLIENT_ID`, were read back from GitHub.

Exactly one federated credential was created and verified:

```text
issuer: https://token.actions.githubusercontent.com
audience: api://AzureADTokenExchange
subject: repo:kc2-io@157092316/MPD_Viewer@1376346660:environment:release-windows
```

The GitHub OIDC settings API reports `use_default=true`,
`use_immutable_subject=true`, and the repository-ID-bearing prefix above. This
supersedes the name-only subject in the earlier handoff. GitHub's
[immutable subject format](https://docs.github.com/en/actions/reference/security/oidc)
must be matched exactly; no repository OIDC settings were changed.

The **Artifact Signing Certificate Profile Signer** role was assigned only at:

```text
/subscriptions/73209fda-257d-4d9e-beae-4b5892d95d90/resourceGroups/MPD-Artifacts/providers/Microsoft.CodeSigning/codeSigningAccounts/MPD-Artifacts/certificateProfiles/mpd-bot-public
```

Azure role-assignment readback found exactly this one assignment for the new
principal in the verified subscription. The active Azure tenant/subscription,
GitHub release-environment protections, signing endpoint, publisher and disabled
release flags were checked before writes. The provisioning script received
independent security review. No paid signing resources, client secrets, broad
roles, or reference-repository trust changes were created.

**Configuration is verified; actual OIDC exchange and MPD Viewer signing are
still untested.** No signing job or release tag was launched to bypass the
release gates. A successful future signed artifact must independently pass
publisher, signature and timestamp verification.

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
`CONTINUATION-2026-09-18.md`; [policy PR #5](https://github.com/kc2-io/MPD_Viewer/pull/5) passed all five
required checks in [run 35389308958](https://github.com/kc2-io/MPD_Viewer/actions/runs/35389308958)
and merged as `1b12fc4e5cdab6c7f1907d75022b2c59c941c745`. No application source, dependency,
branding, stored identity, or hosted HTML changes are included.
