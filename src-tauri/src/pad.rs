use serde::{Deserialize, Serialize};
use std::sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}};
use std::thread::JoinHandle;
use tauri::{AppHandle,Emitter};
use whaletouch_core::Mapping;

#[derive(Clone,Copy,Serialize,Deserialize)]
#[serde(default)]
pub struct Options {pub gain:f32,pub light:bool,pub mapping:Mapping}
impl Default for Options {fn default()->Self{Self{gain:0.45,light:true,mapping:Mapping::default()}}}
struct Session {stop:Arc<AtomicBool>,worker:JoinHandle<()>}
#[derive(Default)]
pub struct Controller {session:Mutex<Option<Session>>,pub options:Arc<Mutex<Options>>}
impl Controller {
    pub fn enabled(&self)->bool {self.session.lock().unwrap().as_ref().is_some_and(|s|!s.worker.is_finished())}
    pub fn disable(&self) {
        if let Some(s)=self.session.lock().unwrap().take(){
            s.stop.store(true,Ordering::Release);
            let _=s.worker.join(); // read_timeout bounds shutdown; handles drop before returning.
        }
    }
    pub fn enable(&self,app:AppHandle)->Result<String,String>{
        self.disable();
        #[cfg(not(windows))] {let _=app;return Err("DualSense 触摸仅支持 Windows USB 连接；当前可用鼠标演示。".into())}
        #[cfg(windows)] {
            use hidapi::HidApi;
            let api=HidApi::new().map_err(|_|"无法枚举 HID 设备")?;
            let info=api.device_list().find(|d| d.vendor_id()==0x054c && [0x0ce6,0x0df2].contains(&d.product_id()) && (d.interface_number()==3 || d.path().to_string_lossy().to_lowercase().contains("mi_03"))).ok_or("未找到 USB 形态 DualSense。请插入 USB 线，蓝牙直连暂不支持。")?;
            let name=if info.product_id()==0x0df2 {"DualSense Edge"}else{"DualSense"}.to_string();
            let device=info.open_device(&api).map_err(|_|"手柄打开失败，请检查其他软件是否独占设备")?;
            let stop=Arc::new(AtomicBool::new(false));let stopping=stop.clone();let options=self.options.clone();
            let worker=std::thread::spawn(move||{
                use std::time::{Duration,Instant};
                use whaletouch_core::{TouchTracker,parse_usb,feedback,output_report};
                let start=Instant::now();let mut tracker=TouchTracker::default();let mut previous=None;let mut last_emit=Instant::now()-Duration::from_secs(1);
                let mut last_active=false;let mut error=None;let mut idle_reads=0u32;
                while !stopping.load(Ordering::Acquire) {
                    let mut bytes=[0u8;128];
                    let size=match device.read_timeout(&mut bytes,50){Ok(0)=>{idle_reads+=1;if idle_reads>100{error=Some("手柄未返回 USB 数据，已释放连接");break}continue},Ok(n)=>{idle_reads=0;n},Err(_)=>{error=Some("手柄已断开，触摸模式已关闭");break}};
                    let point=match parse_usb(&bytes[..size]){Ok(p)=>p,Err(_)=>{error=Some("手柄报文不兼容，请使用 USB 形态连接");break}};
                    let opt=*options.lock().unwrap();let now=start.elapsed().as_millis() as u64;
                    let frame=tracker.update(point,now,opt.mapping);
                    let (weak,strong)=feedback(&frame,opt.gain,now);
                    let rgb=if opt.light&&frame.active {frame.zone.color()}else{[0,0,0]};
                    let output=output_report(weak,strong,rgb,false);
                    if previous.as_ref()!=Some(&output) {
                        if device.write(&output).is_err(){error=Some("手柄输出失败，已释放连接");break}
                        previous=Some(output);
                    }
                    if frame.ended || frame.active!=last_active || (frame.active && last_emit.elapsed()>=Duration::from_millis(33)) {
                        let _=app.emit("pad-frame",&frame);last_emit=Instant::now();
                    }
                    last_active=frame.active;
                }
                // Stop both motors and relinquish LED control; original LED color is not readable.
                let _=device.write(&whaletouch_core::output_report(0,0,[0,0,0],true));
                drop(device);drop(api);
                if let Some(message)=error {let _=app.emit("pad-status",serde_json::json!({"enabled":false,"message":message}));}
            });
            *self.session.lock().unwrap()=Some(Session{stop,worker});Ok(name)
        }
    }
}
impl Drop for Controller {fn drop(&mut self){self.disable();}}
