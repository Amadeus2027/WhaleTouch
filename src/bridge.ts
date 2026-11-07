import {invoke,convertFileSrc} from '@tauri-apps/api/core';
import {listen,emit} from '@tauri-apps/api/event';
import {defaults,type Snapshot,type Settings} from './types';
export const native='__TAURI_INTERNALS__' in window;
const demo:Snapshot={settings:structuredClone(defaults),has_key:false,progress:{companion_seconds:0,feeds:0,usage:{prompt_tokens:0,completion_tokens:0,total_tokens:0},usage_by_day:{}},timer:null,notices:[]};
try{const stored=JSON.parse(localStorage.getItem('whaletouch-preview')||'null');if(stored)demo.settings={...demo.settings,...stored};}catch{}
export async function call<T=void>(command:string,args:Record<string,unknown>={}):Promise<T>{
 if(native)return invoke<T>(command,args);
 switch(command){
  case 'snapshot':return structuredClone(demo) as T;
  case 'save_settings':demo.settings=structuredClone(args.settings as Settings);localStorage.setItem('whaletouch-preview',JSON.stringify(demo.settings));await send('settings-changed',demo.settings);return undefined as T;
  case 'pet_action':if(args.action==='eat')demo.progress.feeds++;await send('pet-action',args.action);return undefined as T;
  case 'record_progress':demo.progress.companion_seconds+=Number(args.seconds||0);if(args.feed)demo.progress.feeds++;return undefined as T;
  case 'window_action':if(args.action==='show')window.open('/?window=pet','whaletouch-pet','width=360,height=560');return undefined as T;
  case 'cancel_chat':return undefined as T;
  default:throw new Error('此功能需要 Windows 桌面版。浏览器预览不连接手柄、不保存密钥，也不调用官方 API。');
 }
}
export async function on<T>(name:string,callback:(data:T)=>void):Promise<()=>void>{
 if(native)return listen<T>(name,e=>callback(e.payload));
 const fn=(e:Event)=>callback((e as CustomEvent).detail);window.addEventListener(name,fn);return()=>window.removeEventListener(name,fn);
}
export async function send<T>(name:string,data:T){if(native)await emit(name,data);else window.dispatchEvent(new CustomEvent(name,{detail:data}));}
export const asset=(path:string)=>native?convertFileSrc(path):path;
