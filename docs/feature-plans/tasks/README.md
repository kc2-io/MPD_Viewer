# Task index

All packets are planned, not executed. Follow dependencies rather than numeric order.

| Task | Work | Agent / effort | Dependencies |
|---|---|---|---|
| [MV-000](MV-000-establish-current-baseline-and-execution-environment.md) | Establish current baseline and execution environment | `mpdv_native` / high | None |
| [MV-010](MV-010-rebrand-visibly-and-replace-the-tagline-exactly.md) | Rebrand visibly and replace the tagline exactly | `mpdv_mechanical` / low | MV-000 |
| [MV-020](MV-020-diagnose-viewer-login-and-the-turbo-ad-observation.md) | Diagnose viewer login and the Turbo ad observation | `mpdv_architect` / high | MV-000 |
| [MV-040](MV-040-prove-native-grid-and-retained-surface-moves.md) | Prove native grid and retained-surface moves | `mpdv_native` / high | MV-000 |
| [MV-001](MV-001-freeze-minimal-authentication-and-presentation-contracts.md) | Freeze minimal authentication and presentation contracts | `mpdv_architect` / high | MV-010, MV-020, MV-040 |
| [MV-021](MV-021-implement-shared-viewer-profile-and-constrained-login-popups.md) | Implement shared viewer profile and constrained login popups | `mpdv_native` / high | MV-001 |
| [MV-030](MV-030-add-official-per-channel-chat-to-the-hosted-and-demo-wrapper.md) | Add official per-channel chat to the hosted and Demo wrapper | `mpdv_web` / medium | MV-001 |
| [MV-022](MV-022-make-api-connection-and-viewer-sign-in-distinct-in-the-manager.md) | Make API connection and viewer sign-in distinct in the manager | `mpdv_web` / medium | MV-010, MV-021 |
| [MV-031](MV-031-version-the-wrapper-protocol-and-preserve-old-clients.md) | Version the wrapper protocol and preserve old clients | `mpdv_native` / high | MV-021, MV-030 |
| [MV-041](MV-041-extract-surface-presentation-while-preserving-standalone-behavior.md) | Extract surface presentation while preserving standalone behavior | `mpdv_native` / high | MV-022, MV-031 |
| [MV-042](MV-042-implement-true-grid-container-and-geometry.md) | Implement true grid container and geometry | `mpdv_native` / high | MV-041 |
| [MV-043](MV-043-implement-safe-layout-transitions-cleanup-and-persistence.md) | Implement safe layout transitions, cleanup, and persistence | `mpdv_native` / high | MV-042 |
| [MV-044](MV-044-expose-layout-selection-and-state-in-the-manager.md) | Expose layout selection and state in the manager | `mpdv_web` / medium | MV-022, MV-043 |
| [MV-050](MV-050-independently-review-security-and-lifecycle-invariants.md) | Independently review security and lifecycle invariants | `mpdv_security` / high | MV-010, MV-031, MV-043, MV-044 |
| [MV-051](MV-051-run-regression-and-real-runtime-acceptance-matrix.md) | Run regression and real-runtime acceptance matrix | `mpdv_qa` / high | MV-050 |
| [MV-060](MV-060-package-compatible-release-and-deployment-handoff.md) | Package compatible release and deployment handoff | `mpdv_native` / high | MV-051 |
