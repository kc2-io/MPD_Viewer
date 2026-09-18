# Source review — evidence behind this plan

Reviewed the mounted `MPD-Tabber-POC-Hosted-Parent.zip` without modifying it. Archive SHA-256: `542075ad3983cc9b1ae819d3220b2f1838859dbd877bbb534fb37b601200fb07`.

These excerpts identify the specific old code to inspect. They do not prove the user is running this exact source, and they do not establish native runtime behavior. The user's actual checkout and deployed assets must be compared in MV-000. No source changes or new application tests were executed for this planning handoff.

## Findings

| Finding in the reviewed source | Planning consequence |
|---|---|
| Visible name/tagline reside in manager HTML and native config/title strings. | A bounded visible rename is enough; avoid a global identifier replacement. |
| Config lookup uses the stable application identity. | Preserve it during the rebrand and verify upgrades. |
| Video wrapper uses Twitch.Player with no chat iframe. | Add official chat beside the existing video rather than rewrite playback by default. |
| New native windows are always denied. | Official sign-in popups need a scoped, tested exception. |
| The current native origin policy does not include the chat origin. | Adding chat HTML alone is insufficient. |
| OAuth activation opens in the system browser. | The API-connected account is not evidence of an embedded player login. |
| Each player is a WebviewWindow and telemetry matches window label to session. | Grid requires surface-level identity and a presenter boundary. |
| Report deserialization denies extra fields. | Negotiate v2; do not break old installed clients by deploying extra report fields. |
| The wrapper is compiled into the local Demo server as exactly three assets. | Asset splitting requires matching server/deploy/test updates. |
| Cargo.lock is absent from this archive. | Obtain the real resolved dependency graph before relying on current docs. |

## Targeted code excerpts

### Visible brand and exact previous tagline

`ui/index.html:3–7`

```text
3: <head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>MPD Tabber</title><link rel="stylesheet" href="style.css"><script type="module" src="app.js"></script></head>
4: <body>
5: <header class="topbar">
6:   <div class="brand"><div class="monogram" aria-hidden="true">M</div><div><h1>MPD Tabber <span class="tag">POC / 0.1</span></h1><p>Your favorites. Your priorities.</p></div></div>
7:   <div class="transport"><span id="run-status" class="pill">Stopped</span><button id="start" class="primary">Start</button><button id="pause">Pause</button><button id="stop" class="danger">Stop</button></div>
```

### Packaged name, stable bundle identifier, main window title

`src-tauri/tauri.conf.json:1–18`

```text
1: {
2:   "$schema": "https://schema.tauri.app/config/2",
3:   "productName": "MPD Tabber POC",
4:   "version": "0.1.0",
5:   "identifier": "com.modpackdad.mpdtabber.poc",
6:   "build": {
7:     "frontendDist": "../ui"
8:   },
9:   "app": {
10:     "withGlobalTauri": true,
11:     "windows": [
12:       {
13:         "label": "main",
14:         "title": "MPD Tabber \u00b7 Proof of Concept",
15:         "width": 1200,
16:         "height": 850,
17:         "minWidth": 860,
18:         "minHeight": 650
```

### Telemetry currently authenticates a report by WebviewWindow label

`src-tauri/src/main.rs:27–36`

```text
27: #[tauri::command]
28: fn player_report(window: WebviewWindow, state: State<'_, Handle>, report: Report) -> Result<(), String> {
29:     if !window.label().starts_with("player-") || window.label() != format!("player-{}", report.session) {
30:         return Err("Session does not belong to this window.".into());
31:     }
32:     if report.volume.is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v)) {
33:         return Err("Invalid volume report.".into());
34:     }
35:     // Bounded queue; hostile subframes cannot allocate an unbounded event backlog.
36:     state.tx.try_send(Message::Report(window.label().into(), report)).map_err(|_| "Telemetry queue is full.".into())
```

### Preferences are resolved via the existing app configuration identity

`src-tauri/src/main.rs:40–46`

```text
40:     tauri::Builder::default()
41:         .invoke_handler(tauri::generate_handler![get_state, dispatch, player_report])
42:         .setup(|app| {
43:             let path = app.path().app_config_dir()?.join("preferences.sqlite3");
44:             let store = storage::Store::open(&path)?;
45:             let settings = store.load()?;
46:             let host = player::Host::start()?;
```

### Standalone construction, origin/navigation policy, and deny-all popups

`src-tauri/src/player.rs:59–71`

```text
59:         WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
60:             .title(format!("{login} · MPD Tabber{}", if settings.demo { " · SIMULATED" } else { "" }))
61:             .inner_size(820.0, 550.0).min_inner_size(430.0, 480.0)
62:             .focused(false)
63:             .on_navigation(move |target| {
64:                 // No navigation to arbitrary sites and no OS custom-scheme launches.
65:                 // Twitch's own player origin is also allowed for iframe navigation.
66:                 target.origin() == origin || (target.scheme() == "https" && target.host_str() == Some("player.twitch.tv"))
67:             })
68:             .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
69:             .on_download(|_, _| false)
70:             .build().map_err(|e| format!("Could not create player: {e}"))?;
71:         Ok(label)
```

### API device authorization uses the normal external browser

`src-tauri/src/controller.rs:178–186`

```text
178:             Action::OpenAuth => {
179:                 let text = self.auth_url.as_ref().ok_or("No authorization is pending.")?;
180:                 let url = url::Url::parse(text).map_err(|_| "Invalid Twitch authorization URL.")?;
181:                 if url.scheme() != "https" || !matches!(url.host_str(), Some("www.twitch.tv" | "twitch.tv"))
182:                     || url.path() != "/activate" || !url.username().is_empty() || url.password().is_some() {
183:                     return Err("Rejected unexpected authorization URL.".into());
184:                 }
185:                 webbrowser::open(url.as_str()).map_err(|_| "Could not open your browser. Visit Twitch's activation page manually.")?;
186:             }
```

### Existing report schema rejects unknown fields

`src-tauri/src/model.rs:74–84`

```text
74: pub enum Playback { Loading, Ready, Buffering, Playing, Paused, Blocked, Offline, Ended, Error }
75: 
76: #[derive(Clone, Serialize, Deserialize)]
77: #[serde(deny_unknown_fields)]
78: pub struct Report {
79:     pub session: u64,
80:     pub state: Playback,
81:     pub visible: bool,
82:     pub volume: Option<f64>,
83:     pub muted: Option<bool>,
84: }
```

### Only advisory telemetry is granted to player windows on approved origins

`src-tauri/capabilities/players.json:1–17`

```text
1: {
2:   "identifier": "players",
3:   "description": "Player wrappers may only report advisory telemetry for themselves.",
4:   "windows": [
5:     "player-*"
6:   ],
7:   "local": false,
8:   "remote": {
9:     "urls": [
10:       "http://localhost:*/*",
11:       "https://parent.mpdviewer.com/*"
12:     ]
13:   },
14:   "permissions": [
15:     "report-playback"
16:   ]
17: }
```

### Management capability is local and scoped to main

`src-tauri/capabilities/manager.json:1–7`

```text
1: {
2:   "identifier": "manager",
3:   "description": "Trusted bundled management interface. No remote origins.",
4:   "windows": ["main"],
5:   "local": true,
6:   "permissions": ["manage"]
7: }
```

### Video-only frame policy and wrapper structure

`player-wrapper/index.html:1–6`

```text
1: <!doctype html>
2: <html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
3: <meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'self' https://player.twitch.tv; style-src 'self'; frame-src https://player.twitch.tv; connect-src ipc: http://ipc.localhost https://ipc.localhost; img-src 'self' data:; base-uri 'none'; form-action 'none'">
4: <title>MPD Player</title><link rel="stylesheet" href="style.css"><script src="player.js" defer></script></head>
5: <body><header><div><strong id="channel">MPD player</strong><span id="kind"></span></div><span id="status">Loading</span></header>
6: <div id="video"></div>
```

### Official video player initialization and one initial play attempt

`player-wrapper/player.js:47–62`

```text
47:     changeState('playing');return;
48:   }
49:   $('try-play').addEventListener('click',()=>{if(ready&&player)player.play();});
50:   const script=document.createElement('script');script.src='https://player.twitch.tv/js/embed/v1.js';
51:   script.onerror=()=>{changeState('error');$('message').textContent='Twitch player SDK could not load. Check the network and configured player origin.';};
52:   script.onload=()=>{
53:     try{
54:       // Never start at the SDK's default volume. Apply requested audio after READY.
55:       player=new Twitch.Player('video',{channel,width:'100%',height:'100%',parent:[location.hostname],autoplay:false,muted:true});
56:       for(const [event,value] of [['PLAY','buffering'],['PLAYING','playing'],['PAUSE','paused'],
57:         ['PLAYBACK_BLOCKED','blocked'],['OFFLINE','offline'],['ENDED','ended']]){
58:         player.addEventListener(Twitch.Player[event],()=>changeState(value));
59:       }
60:       player.addEventListener(Twitch.Player.READY,()=>{ready=true;window.mpdSetAudio(volume,muted);changeState('ready');player.play();});
61:       // ONLINE is not evidence of video playback. Only PLAYING reports playing.
62:     }catch{changeState('error');$('message').textContent='Could not initialize the Twitch embed. Inspect the player error and test a valid HTTPS wrapper origin.';}
```

## Target-file fingerprints

These hashes identify the inspected archive versions, not the current deployment.

| File | SHA-256 |
|---|---|
| `ui/index.html` | `b6584dd2413ffd67c288166ceef5a1053c778ca012721ce1bab57e63ac15de73` |
| `src-tauri/tauri.conf.json` | `8f74da8f896faa8e3a824cd546eadd1f5ad02fc33536fc79f2250bf4b8630e68` |
| `src-tauri/src/main.rs` | `9f021ce0a332f524b5bbc74380b017b3c83c072fc4ab5ddf6db2f6a003933fc3` |
| `src-tauri/src/player.rs` | `2064be0621d0e2259e259d6d90883cf06a88f804183b39fdee0fbaa9faf34310` |
| `src-tauri/src/controller.rs` | `81124a362a886ce306b47f9b76a66364735308f70924857905e8ac9f987a32ce` |
| `src-tauri/src/model.rs` | `dc040ccf1760f7460b43feba9da304f0c877ec31403bce7728f3e6682012d336` |
| `src-tauri/capabilities/players.json` | `0a25f4d7946a178649c1abe4ba575b60b4cd3c6aa64cfaff0f5290b8c8521a3d` |
| `src-tauri/capabilities/manager.json` | `5f340ca90189e477629ae628e6f49f58b6a24aafe0ddca747068035ef4601277` |
| `player-wrapper/index.html` | `8bab8073497bfad9b6ec772346e901b4cf14f24d7b8a344773cf2b613011ac8f` |
| `player-wrapper/player.js` | `4c258ecaa7c9293f1e522702a47a5bdcd5b2dca55e9ce8025e013a9d8f2ab45a` |
