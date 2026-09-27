# CI and release instructions

Read the root `AGENTS.md`, `docs/setup/RELEASING.md`, and
`docs/setup/SIGNING.md` before changing workflows or release policy.

- Pin every action to a reviewed full commit SHA and update the pin manifest.
- Pull-request workflows remain least-privileged, unsigned, secret-free, and
  without release environments or OIDC.
- Privileged `workflow_run` jobs must use trusted default-branch code, validate
  repository/PR/commit association immediately before writes, and never execute
  or check out untrusted PR code.
- Keep native compilation, GUI fixture evidence, signing, packaging, and live
  application acceptance as distinct gates.
- Do not loosen identity, visibility, ancestry, tag, signature, artifact-set, or
  publication checks to pass a release.
- Run `just verify` for policy/source changes and the additional relevant lanes
  from `docs/TEST-MATRIX.md`.
