export type Zone='head'|'neck'|'chest'|'waist'|'hips'|'thigh'|'shin'|'foot';
export type Action='idle'|'touch'|'poke'|'lift'|'land'|'walk'|'eat'|'sleep'|'wake'|'chat'|'focus';
export interface Mapping{chibi:boolean;flip_u:boolean;flip_v:boolean}
export interface Settings{character:'chibi'|'adult';live2d:boolean;model:string;dark:boolean;dnd:boolean;auto_reply:boolean;show_touch:boolean;scale:number;pad:{gain:number;light:boolean;mapping:Mapping};custom_model:string|null}
export interface TouchFrame{active:boolean;u:number;v:number;zone:Zone;speed:number;gesture:string;ended:boolean}
export interface Usage{prompt_tokens:number;completion_tokens:number;total_tokens:number}
export interface Progress{companion_seconds:number;feeds:number;usage:Usage;usage_by_day:Record<string,Usage>}
export interface Timer{id:number;label:string;ends:number;remaining:number;paused:boolean}
export interface Notice{id:number;text:string;at:number}
export interface Snapshot{settings:Settings;has_key:boolean;progress:Progress;timer:Timer|null;notices:Notice[]}
export interface Balance{is_available:boolean;balance_infos:{currency:string;total_balance:string;granted_balance:string;topped_up_balance:string}[]}
export const zoneNames:Record<Zone,string>={head:'头部',neck:'颈肩',chest:'胸部',waist:'腰腹',hips:'臀胯',thigh:'大腿',shin:'小腿',foot:'脚部'};
export const zoneEnds=(chibi:boolean)=>chibi?[.35,.40,.52,.60,.70,.84,.94,1]:[.18,.26,.40,.52,.62,.78,.92,1];
export const zones=Object.keys(zoneNames) as Zone[];
export function mappedFrame(x:number,y:number,s:Settings,active=true):TouchFrame{
 let v=Math.max(0,Math.min(1,x)),u=1-Math.max(0,Math.min(1,y));if(s.pad.mapping.flip_u)u=1-u;if(s.pad.mapping.flip_v)v=1-v;
 return{active,u,v,zone:zones[zoneEnds(s.character==='chibi').findIndex(end=>v<end)]??'foot',speed:35,gesture:active?'stroke':'hold',ended:!active};
}
export const defaults:Settings={character:'chibi',live2d:false,model:'deepseek-flash',dark:false,dnd:false,auto_reply:false,show_touch:true,scale:1,pad:{gain:.45,light:true,mapping:{chibi:true,flip_u:false,flip_v:false}},custom_model:null};
