# Official references

Reviewed September 17, 2026. These sources support platform/model facts; the feature choices and task allocations are architecture recommendations. Recheck version-specific behavior during MV-000 and the feasibility tasks. A documentation page is not a native-runtime acceptance test.

| ID | Reference | Relevant topic |
|---|---|---|
| S1 | [Twitch: Embedding Chat](https://dev.twitch.tv/docs/embed/chat/) | Chat URL, parent, sandbox capabilities. |
| S2 | [Twitch: Embedding Everything](https://dev.twitch.tv/docs/embed/everything/) | Combined embed and official login popup experience. |
| S3 | [Twitch: Video and Clips](https://dev.twitch.tv/docs/embed/video-and-clips/) | Player APIs, dimensions, playback state. |
| S4 | [Twitch: Embed requirements](https://dev.twitch.tv/docs/embed/) | SSL, actual parent, unobscured approved embeds. |
| S5 | [Tauri: WebviewBuilder](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewBuilder.html) | Child-view feature gate, browser data-store options. |
| S6 | [Tauri: Webview](https://docs.rs/tauri/latest/tauri/webview/struct.Webview.html) | Native reparent, bounds, focus, surface APIs. |
| S7 | [Microsoft: WebView2 user-data folders](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/user-data-folder) | Browser state and profile storage. |
| S8 | [Tauri: NewWindowResponse](https://docs.rs/tauri/latest/tauri/webview/enum.NewWindowResponse.html) and [WebviewWindowBuilder](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html) | Popup opener/environment/configuration requirements. |
| S9 | [Tauri: Capabilities](https://v2.tauri.app/security/capabilities/) | Permission boundaries, custom command exposure, Linux iframe limitation. |
| S10 | [OpenAI: Codex models](https://developers.openai.com/codex/models) | Recommended models and account/client availability. |
| S11 | [OpenAI: GPT-6 Astra](https://developers.openai.com/api/docs/models/gpt-6-astra) | Model identifier and reasoning levels. |
| S12 | [OpenAI: GPT-5.6 Sol](https://developers.openai.com/api/docs/models/gpt-5.6-sol) | Model identifier and reasoning levels. |
| S13 | [OpenAI: GPT-5.6 Terra](https://developers.openai.com/api/docs/models/gpt-5.6-terra) | Model identifier and reasoning levels. |
| S14 | [OpenAI: GPT-5.6 Luna](https://developers.openai.com/api/docs/models/gpt-5.6-luna) | Model identifier and reasoning levels. |
| S15 | [OpenAI: Codex subagents](https://developers.openai.com/codex/subagents) | Custom-agent TOML and per-agent model/effort configuration. |
| S16 | [Twitch: Stream Display Ads](https://help.twitch.tv/s/article/stream-display-ads) | Documented Turbo treatment for these Twitch-served ad formats; not evidence of the user's native session. |

The full Turbo guide and the deployed parent site could not be read successfully here. No inference that all embedded Turbo scenarios work is based on those inaccessible pages. The user's active Turbo subscription is user-provided context; its recognition by each viewer remains an acceptance question.
