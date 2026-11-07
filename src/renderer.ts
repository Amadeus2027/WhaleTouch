import type {Action,Settings,TouchFrame} from './types';
import {call,native,asset} from './bridge';
export interface CharacterRenderer{load(s:Settings):Promise<void>;draw(action:Action,elapsed:number,touch:TouchFrame|null):void;dispose():void;hit?(u:number,v:number):boolean}
export class SpriteRenderer implements CharacterRenderer{
 private image:HTMLImageElement|null=null;private idle:HTMLImageElement|null=null;private adult=false;private hitMask:Uint8ClampedArray|null=null;private maskCanvas=document.createElement('canvas');
 constructor(private canvas:HTMLCanvasElement){}
 async load(s:Settings){this.dispose();this.adult=s.character==='adult';this.image=await this.imageAt(`/characters/${s.character}/actions.png`);if(!this.adult)this.idle=await this.imageAt('/characters/chibi/idle.png');}
 private imageAt(src:string):Promise<HTMLImageElement>{return new Promise((resolve,reject)=>{const i=new Image();i.onload=()=>resolve(i);i.onerror=()=>reject(new Error('角色图片加载失败'));i.src=src;});}
 draw(action:Action,elapsed:number,touch:TouchFrame|null){
  const ctx=this.canvas.getContext('2d');if(!ctx||!this.image)return;
  const w=this.canvas.width,h=this.canvas.height;ctx.clearRect(0,0,w,h);
  let frame=0,cols=4,rows=this.adult?3:4,img=this.image;
  if(this.adult){frame=({idle:0,touch:({head:1,neck:2,chest:3,waist:2,hips:3,thigh:11,shin:11,foot:11} as const)[touch?.zone??'head'],poke:3,lift:10,land:11,walk:Math.floor(elapsed/220)%2?8:9,eat:4,sleep:7,wake:5,chat:5,focus:6} satisfies Record<Action,number>)[action];}
  else{frame=({idle:0,touch:({head:1,neck:3,chest:4,waist:2,hips:8,thigh:14,shin:15,foot:15} as const)[touch?.zone??'head'],poke:8,lift:14,land:15,walk:Math.floor(elapsed/240)%2?12:13,eat:6,sleep:11,wake:4,chat:10,focus:2} satisfies Record<Action,number>)[action];if(action==='idle'&&this.idle){img=this.idle;cols=2;rows=1;frame=0;}}
  const sw=img.naturalWidth/cols,sh=img.naturalHeight/rows;
  const size=this.adult?Math.min(w*.95,h*.96*sw/sh):Math.min(w*1.12,h*.90);
  const dh=size*sh/sw;const x=(w-size)/2,y=h-dh-16;
  const wave=action==='walk'?Math.sin(Math.min(1,elapsed/2400)*Math.PI)*22+Math.sin(elapsed/90)*2:action==='touch'?Math.sin(elapsed/300)*1.4:0;
  const bounce=action==='land'?Math.max(0,1-elapsed/600)*Math.sin(elapsed/60)*.045:0;
  ctx.save();ctx.translate(w/2,h-16);ctx.rotate(action==='lift'?Math.sin(elapsed/250)*.035:touch?.active?(touch.u-.5)*.025:0);ctx.scale(1+bounce,1-bounce);ctx.translate(-w/2,-h+16);
  ctx.drawImage(img,(frame%cols)*sw,Math.floor(frame/cols)*sh,sw,sh,x+wave,y,size,dh);ctx.restore();
  this.maskCanvas.width=48;this.maskCanvas.height=72;const mask=this.maskCanvas.getContext('2d',{willReadFrequently:true})!;mask.drawImage(this.canvas,0,0,48,72);this.hitMask=mask.getImageData(0,0,48,72).data;
 }
 hit(u:number,v:number){if(!this.hitMask||u<0||u>=1||v<0||v>=1)return false;return this.hitMask[(Math.floor(v*72)*48+Math.floor(u*48))*4+3]>35;}
 dispose(){this.hitMask=null;if(this.image)this.image.src='';if(this.idle)this.idle.src='';this.image=null;this.idle=null;}
}
export class LiveRenderer implements CharacterRenderer{
 private app:any;private model:any;private before:(()=>void)|null=null;private action:Action='idle';private touch:TouchFrame|null=null;
 constructor(private canvas:HTMLCanvasElement){}
 async load(s:Settings){
  const win=window as any;
  if(!win.Live2DCubismCore){
   const js=await call<string>('get_cubism_core');const script=document.createElement('script');script.textContent=js;document.head.append(script);
  }
  const PIXI=await import('pixi.js');win.PIXI=PIXI;
  const {Live2DModel}=await import('pixi-live2d-display/cubism4');
  this.app=new PIXI.Application({view:this.canvas,width:this.canvas.width,height:this.canvas.height,transparent:true,autoStart:false,antialias:true});
  const source=s.custom_model?(native?asset(s.custom_model):s.custom_model):'/characters/live2d/c_0120.model3.json';
  this.model=await Live2DModel.from(source,{autoInteract:false,autoUpdate:false});
  const k=Math.min(this.canvas.width/this.model.width,this.canvas.height/this.model.height)*.94;
  this.model.scale.set(k);this.model.anchor.set(.5,1);this.model.position.set(this.canvas.width/2,this.canvas.height-15);this.app.stage.addChild(this.model);
  this.model.internalModel.motionManager.stopAllMotions();
  this.before=()=>{
   const core=this.model.internalModel.coreModel;const touch=this.touch;
   const param=(name:string,value:number)=>{if(core.getParameterIndex(name)>=0)core.setParameterValueById(name,value);};
   param('ParamAngleX',touch?.active?(touch.u-.5)*22:0);param('ParamAngleZ',touch?.active?(touch.u-.5)*8:0);
   param('ParamEyeLOpen',this.action==='sleep'?0:touch?.active && touch.zone==='head' ? 0.4 : 1);
   param('ParamEyeROpen',this.action==='sleep'?0:touch?.active && touch.zone==='head' ? 0.4 : 1);
   param('ParamMouthOpenY',this.action==='chat' ? 0.18 : 0);
   param('ParamBodyAngleZ',touch?.active?(touch.u-.5)*5:0);
  };
  this.model.internalModel.on('beforeModelUpdate',this.before);
 }
 draw(action:Action,elapsed:number,touch:TouchFrame|null){
  if(!this.model)return;this.action=action;this.touch=touch;this.model.update(33);this.app.renderer.render(this.app.stage);
 }
 dispose(){if(this.before&&this.model)this.model.internalModel.off('beforeModelUpdate',this.before);this.app?.destroy(false,{children:true,texture:true,baseTexture:true});this.model=null;this.app=null;}
}
