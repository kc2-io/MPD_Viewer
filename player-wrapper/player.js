'use strict';
(() => {
  const params=new URLSearchParams(location.hash.slice(1));
  const channel=params.get('channel')??'';
  const session=Number(params.get('session'));
  const demo=params.get('demo')==='true';
  let volume=Number(params.get('volume')??25), muted=params.get('muted')==='true';
  let player=null, ready=false, state='loading', frozen=false;
  const $=id=>document.getElementById(id);
  if(!/^[a-z0-9_]{1,25}$/.test(channel)||!Number.isSafeInteger(session)||session<1){
    $('message').textContent='Invalid channel or session configuration.';$('status').textContent='Configuration error';return;
  }
  mpdInstallChat({channel, demo, mode: 'bundled'});
  volume=Number.isFinite(volume)?Math.max(0,Math.min(100,volume)):25;
  $('channel').textContent=channel;$('demo-channel').textContent=channel;
  $('kind').textContent=demo?'DEMO · no live video':'Twitch embedded player';
  $('origin').textContent=`Parent: ${location.hostname} · ${location.protocol}`;
  async function report() {
    if(frozen)return;
    const invoke=window.__TAURI__?.core?.invoke;
    if(!invoke){$('bridge').textContent='Standalone page · no native telemetry';return;}
    let actualVolume=demo?volume/100:null,actualMuted=demo?muted:null;
    if(ready&&player){try{actualVolume=player.getVolume();actualMuted=player.getMuted();}catch{}}
    try{
      await invoke('player_report',{report:{session,state,visible:document.visibilityState==='visible',
        volume:Number.isFinite(actualVolume)?actualVolume:null,muted:typeof actualMuted==='boolean'?actualMuted:null}});
      $('bridge').textContent='Native telemetry connected';
    }catch{$('bridge').textContent='Telemetry blocked · check player capability origin';}
  }
  function changeState(next){state=next;$('status').textContent=`${demo?'Simulated · ':''}${next}`;
    if(next==='blocked')$('message').textContent='Autoplay was blocked. Click Play inside the Twitch player. MPD will not repeatedly unpause it.';
    report();}
  window.mpdSetAudio=(nextVolume,nextMuted)=>{
    if(!Number.isFinite(nextVolume)||nextVolume<0||nextVolume>100||typeof nextMuted!=='boolean')return;
    volume=nextVolume;muted=nextMuted;
    if(ready&&player){try{player.setVolume(volume/100);player.setMuted(muted);}catch{changeState('error');}}
    $('audio').textContent=`Requested volume ${volume}% · ${muted?'muted':'unmuted'}`;
    report();
  };
  window.mpdSetAudio(volume,muted);
  document.addEventListener('visibilitychange',report);
  setInterval(report,5000); // Observation only: never calls play or changes visibility.
  if(demo){
    $('video').hidden=true;$('demo').hidden=false;$('try-play').hidden=true;
    $('message').textContent='Use these controls to test telemetry. Minimize this window to test visibility reporting.';
    for(const b of document.querySelectorAll('[data-state]'))b.addEventListener('click',()=>changeState(b.dataset.state));
    $('freeze').addEventListener('click',()=>{frozen=!frozen;$('freeze').textContent=frozen?'Resume telemetry':'Pause telemetry';if(!frozen)report();});
    changeState('playing');return;
  }
  $('try-play').addEventListener('click',()=>{if(ready&&player)player.play();});
  const script=document.createElement('script');script.src='https://player.twitch.tv/js/embed/v1.js';
  script.onerror=()=>{changeState('error');$('message').textContent='Twitch player SDK could not load. Check the network and configured player origin.';};
  script.onload=()=>{
    try{
      // Never start at the SDK's default volume. Apply requested audio after READY.
      player=new Twitch.Player('video',{channel,width:'100%',height:'100%',parent:[location.hostname],autoplay:false,muted:true});
      for(const [event,value] of [['PLAY','buffering'],['PLAYING','playing'],['PAUSE','paused'],
        ['PLAYBACK_BLOCKED','blocked'],['OFFLINE','offline'],['ENDED','ended']]){
        player.addEventListener(Twitch.Player[event],()=>changeState(value));
      }
      player.addEventListener(Twitch.Player.READY,()=>{ready=true;window.mpdSetAudio(volume,muted);changeState('ready');player.play();});
      // ONLINE is not evidence of video playback. Only PLAYING reports playing.
    }catch{changeState('error');$('message').textContent='Could not initialize the Twitch embed. Inspect the player error and test a valid HTTPS wrapper origin.';}
  };
  document.head.append(script);
})();
