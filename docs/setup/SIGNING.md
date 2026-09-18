# Signing configuration — Azure trust provisioned; signing untested

Azure application/service-principal creation, exact GitHub OIDC trust, the existing certificate-profile-scoped signer role, and all seven Windows environment variables were provisioned and read back on September 18, 2026. See [the current setup record](PUBLIC-SETUP-2026-09-18.md). Actual GitHub OIDC exchange and MPD Viewer signing have **not** been tested. Apple credentials are unavailable; macOS signing/notarization remain pending. Secrets must be entered in the appropriate provider/secret settings, never pasted into chat or committed.

## Windows: Azure Artifact Signing + OIDC

This is the prepared Windows implementation. Compare it with the current BotOrNot/mpd-bot workflows before adopting the exact variable names. Reuse an existing authorized signing account/profile where appropriate; this package does not purchase or create an Azure signing resource.

Environment: **release-windows**. Use environment Actions variables:

| Variable | Value to supply |
|---|---|
| AZURE_CLIENT_ID | Authorized Entra application's client ID |
| AZURE_TENANT_ID | Directory ID |
| AZURE_SUBSCRIPTION_ID | Subscription ID |
| AZURE_SIGNING_ENDPOINT | Endpoint of the actual signing account's region |
| AZURE_SIGNING_ACCOUNT | Existing authorized signing account name |
| AZURE_CERTIFICATE_PROFILE | Existing code-signing certificate profile name |
| WINDOWS_SIGNER_SUBJECT | Exact expected certificate subject from a verified existing signed binary/profile |

These IDs/configuration values are not client secrets. No Azure client secret or exported private key is required for the proposed OIDC path.

Create a narrowly scoped Entra federated identity credential through an authorized administrator:

```text
issuer:   https://token.actions.githubusercontent.com
audience: api://AzureADTokenExchange
subject:  repo:kc2-io@157092316/MPD_Viewer@1376346660:environment:release-windows
```

The live repository OIDC API reports immutable subjects and the prefix above; its owner and repository IDs were also verified. Do not substitute the older name-only subject. Recheck the live OIDC configuration if repository identity changes. An environment-based OIDC subject does not contain the tag; the GitHub environment's tag deployment policy, release workflow gates, and trusted main ancestry jointly restrict the context. Assign the **Artifact Signing Certificate Profile Signer** role at the narrow required profile/account scope. Do not broaden existing BotOrNot trust to arbitrary repositories or refs.

Only the Windows signing job requests an OIDC token. It receives the binary from the same workflow run, signs and timestamps it, then independently checks Authenticode status, exact publisher subject and timestamp before making the ZIP. No compilation runs after signing. No MSI/NSIS installer is produced in this first setup.

A successful signature is not a guarantee about Windows reputation or SmartScreen prompts. Verify the downloaded executable independently. Azure authentication setup, certificate policy and actual access must be tested; do not infer success from the presence of variables.

## macOS: Developer ID + notarization

Environment: **release-macos**. Uses the existing authorized Apple developer team; no Apple account or paid enrollment is created.

| Kind | Name | Content |
|---|---|---|
| Secret | APPLE_CERTIFICATE | Strict base64-encoded Developer ID Application P12 with its private key |
| Secret | APPLE_CERTIFICATE_PASSWORD | P12 export password |
| Secret | APPLE_API_KEY_P8 | App Store Connect notary API private key, original PEM text |
| Variable | APPLE_SIGNING_IDENTITY | Exact `Developer ID Application: ... (TEAMID)` identity |
| Variable | APPLE_TEAM_ID | Expected Apple team ID |
| Variable | APPLE_API_KEY_ID | Notary API key ID |
| Variable | APPLE_API_ISSUER | Issuer UUID for that API key |

Use an API key supported by `notarytool` with its documented permissions. The implementation targets the issuer-based key flow, not an unverified individual-key variation. Do not supply an Apple account password as a substitute.

The signing script imports into a temporary keychain; packages the already-built binary; verifies the app's Developer ID signature; submits the app ZIP to notarytool; staples/checks the app; assembles, signs, notarizes and staples the DMG. It restores/removes the temporary keychain and temporary credential files on exit. Credential-bearing commands are not printed. Apple Silicon and Intel use separate native jobs.

Actual certificate chain, hardened-runtime entitlements, Tauri packaging, API-key permission, notarization and Gatekeeper behavior need native acceptance. A successful mocked/static test is not evidence that Apple accepted the app.

## Linux and updater distinctions

Linux DEB/AppImage outputs carry SHA-256 checksums but no OS-native signing identity in this first pipeline. Checksums detect file mismatch; they are not independent publisher signatures. Artifact provenance is recorded with the public GitHub release and build metadata until a separately approved detached-signature/attestation mechanism is added.

No Tauri updater plugin or update-signing key is configured. Updater signatures are a different system from Windows Authenticode, Apple code signing and notarization. No updater key is fabricated or committed and no private-release authentication tokens are distributed with the application.

## Failures and secret handling

Missing variables, rejected signatures, mismatched publisher/team, notarization rejection or an incomplete matrix stop publication. PR/main diagnostic builds remain unsigned and never receive signing credentials. Do not add `continue-on-error`, insecure credential fallbacks, certificate-validation bypasses, or manual unsigned release uploads to make a red job look successful.

Existing GitHub secret values cannot be recovered through a read API and are not copied by this setup. Authorize the new repository for an existing narrowly scoped organization secret, or set the value securely from its original authorized source. Keep the reference repositories unchanged unless separately requested.

## Official references

- [Azure Artifact Signing action and OIDC recommendation](https://github.com/Azure/artifact-signing-action)
- [GitHub OIDC with Azure](https://docs.github.com/en/actions/how-tos/secure-your-work/security-harden-deployments/oidc-in-azure)
- [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/)
- [Tauri Windows signing](https://v2.tauri.app/distribute/sign/windows/)
- [Apple notarization workflow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow)
