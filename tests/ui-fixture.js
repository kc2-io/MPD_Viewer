// Test fixture, NOT the Rust controller. Never shipped with the management UI.
(() => {
  const names=['alpha_demo','bravo_demo','charlie_demo','delta_demo'];
  const settings={schema:1,favorites:names.map(login=>({login,enabled:true})),limit:3,rescan_minutes:1,volume:25,muted:false,demo:true};
  const s={mode:'running',settings,favorites:names.map((login,i)=>({login,enabled:true,presence:i?'live':'offline',skipped:false,demo_live:!!i,open_error:null})),
    players:names.slice(1).map((login,i)=>({login,session:i+1,closing:false,state:'playing',report_age_seconds:0,visible:true,volume:.25,muted:false,window_muted:null,window_mute_override:null})),
    connected_as:null,auth_pending:false,user_code:null,last_check_seconds:12,polling:false,next_check_seconds:18,
    viewer:{backend:'bundled-demo',telemetry:true,media_controls:true,window_mute_controls:false,twitch_channel_page:false},
    player_origin:'http://localhost:49152/index.html',error:null,events:['+12s  Opened delta_demo (session 3).','+12s  Opened charlie_demo (session 2).','+12s  Opened bravo_demo (session 1).','+11s  Monitoring started.','+0s  Demo loaded.']};
  window.__fixture=s;window.__actions=[];
  window.__TAURI__={core:{invoke:async(command,args)=>{
    if(command==='get_state')return structuredClone(s);
    if(command==='dispatch'){
      const a=args.action;window.__actions.push(a);
      if(a.type==='set_window_mute'&&window.__rejectWindowMute)throw new Error('Simulated native mute failure');
      if(a.type==='set_audio'){s.settings.volume=a.volume;s.settings.muted=a.muted;}
      if(a.type==='set_window_mute'){
        if(a.session==null){s.settings.muted=a.muted;for(const p of s.players)if(p.window_muted!=null)p.window_muted=a.muted||p.window_mute_override;}
        else {const p=s.players.find(p=>p.session===a.session);if(p){p.window_mute_override=a.muted;p.window_muted=a.muted;}}
      }
      if(a.type==='set_limit')s.settings.limit=a.limit;
      if(a.type==='set_rescan')s.settings.rescan_minutes=a.minutes;
      if(a.type==='pause')s.mode='paused';
      if(a.type==='start')s.mode='running';
      if(a.type==='stop'){s.mode='stopped';s.players=[];}
      if(a.type==='add'){
        if(a.input.includes('!'))throw 'Rejected test input';
        const login=a.input.toLowerCase();s.favorites.push({login,enabled:true,presence:'unknown',skipped:false,demo_live:false,open_error:null});s.settings.favorites.push({login,enabled:true});
      }
      if(a.type==='move'){
        for(const list of [s.favorites,s.settings.favorites]){const i=list.findIndex(x=>x.login===a.login);const [v]=list.splice(i,1);list.splice(a.position,0,v);}
      }
      if(a.type==='clear_error')s.error=null;
      return;
    }
    throw `Unexpected test command ${command}`;
  }}};
})();
