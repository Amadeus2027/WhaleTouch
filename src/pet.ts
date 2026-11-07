import {call,native,on,send} from './bridge';
import {SpriteRenderer,LiveRenderer,type CharacterRenderer} from './renderer';
import {mappedFrame,zoneNames,type Action,type Settings,type Snapshot,type TouchFrame} from './types';
import {getCurrentWindow,cursorPosition} from '@tauri-apps/api/window';
import {icon} from './icons';

export async function startPet(){
 document.body.className='pet-body';
 document.querySelector('#app')!.innerHTML=`<main class="pet-shell"><div class="pet-bubble" role="status">今天也一起摸会儿鱼吧。</div><canvas id="pet-canvas" aria-label="DeepSeek 桌宠，右键打开控制面板"></canvas><div class="touch-orb"></div><div class="pet-tools"><button id="open-panel" title="控制面板">${icon('settings',17)}</button><button id="hide-pet" title="隐藏并释放手柄">${icon('eye',17)}</button></div><span class="pet-zone"></span></main>`;
 const canvas=document.querySelector<HTMLCanvasElement>('#pet-canvas')!;const bubble=document.querySelector<HTMLElement>('.pet-bubble')!;
 const shell=document.querySelector<HTMLElement>('.pet-shell')!;const orb=document.querySelector<HTMLElement>('.touch-orb')!;
 const snap=await call<Snapshot>('snapshot');let settings=snap.settings;let renderer:CharacterRenderer|null=null;
 let action:Action='idle',started=performance.now(),activeUntil=0,lastInteraction=performance.now(),frame:TouchFrame|null=null,raf=0,loading=0,lastFrame=0;
 let visible=true,pointerDown=false,clickThrough=false,lastHit=true,ignore=false,dragging=false;let hideBubble:ReturnType<typeof setTimeout>|undefined;
 let autoAt=0,companionAt=Date.now();
 function say(text:string,always=false){if(settings.dnd&&!always)return;bubble.textContent=text;bubble.classList.add('visible');clearTimeout(hideBubble);hideBubble=setTimeout(()=>bubble.classList.remove('visible'),3500);}
 function draw(now:number){
  raf=0;if(!visible||document.hidden||!renderer)return;
  if(now-lastFrame<1000/30){raf=requestAnimationFrame(draw);return}lastFrame=now;
  if(activeUntil&&now>activeUntil){if(action!=='sleep'&&action!=='focus')action='idle';activeUntil=0;started=now;}
  renderer.draw(action,now-started,frame);
  if(!frame?.active && (action==='idle'||((action==='focus'||action==='sleep')&&!activeUntil)))return;
  if(now-lastInteraction<30000 || frame?.active || (activeUntil&&now<activeUntil))raf=requestAnimationFrame(draw);
 }
 function wake(){lastInteraction=performance.now();if(!raf)raf=requestAnimationFrame(draw);}
 function act(next:Action){action=next;started=performance.now();activeUntil=next==='sleep'||next==='focus'?started+1800:started+({walk:2400,eat:2200,lift:15000,land:650,wake:1300,chat:1500,touch:1300,poke:900,idle:1000}[next]??1200);wake();}
 async function load(s:Settings){
  const id=++loading;settings=s;document.documentElement.dataset.theme=s.dark?'dark':'light';
  if(raf)cancelAnimationFrame(raf);raf=0;renderer?.dispose();renderer=null;
  // A canvas cannot switch between WebGL and 2D contexts; replace it when swapping engines.
  const nextCanvas=document.createElement('canvas');nextCanvas.id='pet-canvas';nextCanvas.setAttribute('aria-label','DeepSeek 桌宠');
  shell.querySelector('canvas')!.replaceWith(nextCanvas);
  const r=Math.min(devicePixelRatio,1.5);nextCanvas.width=Math.round(360*r);nextCanvas.height=Math.round(520*r);
  shell.style.setProperty('--pet-scale',String(s.scale));
  const next=s.character==='adult'&&s.live2d?new LiveRenderer(nextCanvas):new SpriteRenderer(nextCanvas);
  try{await next.load(s);if(id!==loading){next.dispose();return}renderer=next;wake();}
  catch(error){next.dispose();say(String(error),true);if(s.live2d){const fresh=nextCanvas.cloneNode() as HTMLCanvasElement;nextCanvas.replaceWith(fresh);const r2=new SpriteRenderer(fresh);await r2.load(s);renderer=r2;wake();}}
 }
 async function progress(){const now=Date.now();const seconds=visible?Math.floor((now-companionAt)/1000):0;companionAt=now;if(seconds>0)await call('record_progress',{seconds:Math.min(300,seconds),feed:false});}
 function touch(f:TouchFrame){
  frame=f;orb.style.left=`${12+f.u*76}%`;orb.style.top=`${5+f.v*85}%`;orb.classList.toggle('active',f.active&&settings.show_touch);
  const z=shell.querySelector<HTMLElement>('.pet-zone')!;z.textContent=f.active?zoneNames[f.zone]:'';
  if(f.active){if(action!=='touch')act('touch');else wake();}
  else{act(f.gesture==='tap'&&f.zone==='head'?'poke':'idle');
   if(f.ended&&settings.auto_reply&&!settings.dnd&&Date.now()-autoAt>20000){autoAt=Date.now();void send('interaction-summary',{zone:f.zone,gesture:f.gesture});}
  }
 }
 await on<TouchFrame>('pad-frame',touch);
 await on<TouchFrame>('preview-touch',touch);
 await on<{enabled:boolean}>('pad-status',s=>{if(!s.enabled){frame=null;orb.classList.remove('active');act('idle');}});
 await on<Settings>('settings-changed',s=>{const changed=s.character!==settings.character||s.live2d!==settings.live2d||s.custom_model!==settings.custom_model;settings=s;document.documentElement.dataset.theme=s.dark?'dark':'light';shell.style.setProperty('--pet-scale',String(s.scale));if(changed)void load(s);else wake();});
 await on<string>('pet-action',a=>{
  if(a==='clickthrough'){clickThrough=!clickThrough;if(native){void getCurrentWindow().setIgnoreCursorEvents(clickThrough);clearTimeout(hitTimer);lastHit=true;if(!clickThrough)void hitCheck();}say(clickThrough?'已开启穿透，右键托盘可恢复。':'已恢复鼠标互动。',true);return;}
  if(a in {idle:1,touch:1,poke:1,lift:1,land:1,walk:1,eat:1,sleep:1,wake:1,chat:1,focus:1})act(a as Action);
  if(a==='eat'){say('白饭才是小鲸鱼的能量来源！');void call('record_progress',{seconds:0,feed:true});}
 });
 await on<boolean>('pet-visible',v=>{void progress();visible=v;companionAt=Date.now();if(!v){frame=null;renderer?.dispose();renderer=null;if(raf)cancelAnimationFrame(raf);raf=0;}else void load(settings);});
 await on<{text:string}>('timer-done',n=>say(n.text));
 await on<{delta:string;done:boolean}>('chat-chunk',c=>{if(c.delta){act('chat');}if(c.done)act('idle');});
 shell.addEventListener('contextmenu',e=>{e.preventDefault();void call('window_action',{action:'panel'});});
 shell.addEventListener('pointerdown',e=>{if((e.target as HTMLElement).closest('button'))return;pointerDown=true;dragging=false;shell.setPointerCapture(e.pointerId);if(e.button===0)touch(mappedFrame((e.clientY/innerHeight),(1-e.clientX/innerWidth),settings));});
 let downX=0,downY=0;shell.addEventListener('pointerdown',e=>{downX=e.clientX;downY=e.clientY;});
 shell.addEventListener('pointermove',async e=>{
  if(!pointerDown)return;
  if(native&&e.buttons===1&&!e.shiftKey&&Math.hypot(e.clientX-downX,e.clientY-downY)>7&&!dragging){dragging=true;act('lift');try{await getCurrentWindow().startDragging();}finally{pointerDown=false;frame=null;orb.classList.remove('active');act('land');}return;}
  if(!dragging)touch(mappedFrame(e.clientY/innerHeight,1-e.clientX/innerWidth,settings));
 });
 const up=()=>{pointerDown=false;if(frame?.active)touch({...frame,active:false,ended:true,gesture:'stroke'});};shell.addEventListener('pointerup',up);shell.addEventListener('pointercancel',up);
 document.querySelector('#open-panel')!.addEventListener('click',()=>void call('window_action',{action:'panel'}));
 document.querySelector('#hide-pet')!.addEventListener('click',()=>void call('window_action',{action:'hide'}));
 document.addEventListener('visibilitychange',()=>{if(document.hidden){if(raf)cancelAnimationFrame(raf);raf=0;}else wake();});
 // Pointer hit-test only while visible; no continuous GPU pixel readback.
 let hitTimer:ReturnType<typeof setTimeout>|undefined;
 async function hitCheck(){
  if(!native||!visible||document.hidden||clickThrough)return;
  const win=getCurrentWindow();const [cursor,pos,size]=await Promise.all([cursorPosition(),win.outerPosition(),win.outerSize()]);
  const x=(cursor.x-pos.x)/size.width,y=(cursor.y-pos.y)/size.height;
  const toolHit=x>.72&&x<.98&&y>.9&&y<.99;
  const hit=pointerDown||toolHit||(renderer?.hit?renderer.hit(x,(y-.06)/.94):(x>.16&&x<.84&&y>.10&&y<.96));
  if(hit!==lastHit){lastHit=hit;ignore=!hit;await win.setIgnoreCursorEvents(ignore);}
  hitTimer=setTimeout(()=>void hitCheck(),hit?100:250);
 }
 await load(settings);void hitCheck();
 const companionTimer=setInterval(()=>void progress(),60000);
 window.addEventListener('pagehide',()=>{clearInterval(companionTimer);clearTimeout(hitTimer);clearTimeout(hideBubble);renderer?.dispose();});
}
