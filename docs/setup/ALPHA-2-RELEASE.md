# MPD Viewer v0.1.0-alpha.2

The owner requested a new release after PR #13 was merged. Continue the approved signed-Windows-only alpha scope: signed and timestamped Windows x64 executable in a ZIP, source/player-source archives, metadata and checksums. Keep stable releases disabled and preserve all four native build requirements, signing verification and publication gates. No website deployment.

Base main commit: a8c032d31995f4f165018cf2b9bd3f7217e567ad. This preparation updates workspace/Tauri versions and Cargo-generated workspace lockfile entries to 0.1.0-alpha.2, plus release notes. No dependency, application identity or release-gate changes are intended.

Since alpha.1: MPD logo/colors; unified Windows Twitch connection with fixed public app ID; real activation-link compatibility; constrained identity-server/frontend navigation; an exact validated return URL; automatic closing after authorization and return navigation complete. Per-channel chat and existing settings/profile identity remain intact.

User confirmed activation completed after the return fix. Earlier user observations confirmed shared website-session recognition and restart persistence. Automatic window closing has passed unit/lifecycle checks and review, but its complete real-user acceptance test remains pending. Grid mode is not implemented. API credentials remain in memory; fresh-profile login/MFA, chat posting, Turbo and the known native volume mismatch remain acceptance limits. Do not describe build success as evidence for those behaviors.

Release execution: merge the independently reviewed version PR only after required checks pass; tag clean main equal to origin/main with an immutable annotated v0.1.0-alpha.2; verify all native builds, Azure signature/publisher/timestamp and signed ZIP before approving publication; verify the published download hashes and signature. Record actual run, tag and verification evidence after execution.
