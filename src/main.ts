import './style.css';
const pet=new URLSearchParams(location.search).get('window')==='pet';
async function boot(){if(pet){const {startPet}=await import('./pet');await startPet();}else{const {startPanel}=await import('./panel');await startPanel();}}
boot().catch(error=>{const target=document.querySelector('#app')!;target.textContent=`WhaleTouch 启动失败：${String(error)}`;console.error(error);});
