#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod api;
mod pad;
mod vault;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use whaletouch_core::{Balance, Progress, Usage};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub character: String,
    pub live2d: bool,
    pub model: String,
    pub dark: bool,
    pub dnd: bool,
    pub auto_reply: bool,
    pub show_touch: bool,
    pub scale: f64,
    pub pad: pad::Options,
    pub custom_model: Option<String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            character: "chibi".into(),
            live2d: false,
            model: "deepseek-flash".into(),
            dark: false,
            dnd: false,
            auto_reply: false,
            show_touch: true,
            scale: 1.0,
            pad: pad::Options::default(),
            custom_model: None,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Notice {
    id: u64,
    text: String,
    at: u64,
}
#[derive(Clone, Serialize, Deserialize)]
struct Timer {
    id: u64,
    label: String,
    ends: u64,
    remaining: u64,
    paused: bool,
}
pub struct AppState {
    dir: PathBuf,
    pub client: reqwest::Client,
    pub settings: Mutex<Settings>,
    pub pad: pad::Controller,
    pub balance: Mutex<Option<(Instant, Balance)>>,
    pub chat_cancel: tokio::sync::watch::Sender<u64>,
    progress: Mutex<Progress>,
    timer: Mutex<Option<Timer>>,
    timer_changed: tokio::sync::watch::Sender<u64>,
    notices: Mutex<Vec<Notice>>,
}
fn epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn persist<T: Serialize>(path: PathBuf, value: &T) -> Result<(), String> {
    std::fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|_| "配置编码失败")?,
    )
    .map_err(|_| "无法写入本地配置".into())
}
fn read<T: serde::de::DeserializeOwned + Default>(path: PathBuf) -> T {
    std::fs::read(path)
        .ok()
        .and_then(|x| serde_json::from_slice(&x).ok())
        .unwrap_or_default()
}
impl AppState {
    fn record_usage(&self, u: &Usage) {
        let mut p = self.progress.lock().unwrap();
        fn add(a: &mut Usage, b: &Usage) {
            a.prompt_tokens += b.prompt_tokens;
            a.completion_tokens += b.completion_tokens;
            a.total_tokens += b.total_tokens;
        }
        add(&mut p.usage, u);
        add(
            p.usage_by_day
                .entry(chrono::Utc::now().format("%Y-%m-%d").to_string())
                .or_default(),
            u,
        );
        let _ = persist(self.dir.join("progress.json"), &*p);
    }
}
#[tauri::command]
fn snapshot(app: AppHandle) -> Value {
    let s = app.state::<AppState>();
    json!({"pad_enabled":s.pad.enabled(),"settings":*s.settings.lock().unwrap(),"has_key":s.dir.join("key.bin").exists(),"progress":*s.progress.lock().unwrap(),"timer":*s.timer.lock().unwrap(),"notices":*s.notices.lock().unwrap()})
}
#[tauri::command]
fn save_settings(app: AppHandle, mut settings: Settings) -> Result<(), String> {
    settings.scale = settings.scale.clamp(0.65, 1.4);
    settings.pad.gain = settings.pad.gain.clamp(0.0, 1.0);
    settings.pad.mapping.chibi = settings.character == "chibi";
    let s = app.state::<AppState>();
    persist(s.dir.join("settings.json"), &settings)?;
    *s.pad.options.lock().unwrap() = settings.pad;
    *s.settings.lock().unwrap() = settings.clone();
    let _ = app.emit("settings-changed", settings);
    Ok(())
}
#[tauri::command]
fn save_key(app: AppHandle, key: String) -> Result<(), String> {
    let s = app.state::<AppState>();
    vault::save(&s.dir.join("key.bin"), key.trim())?;
    *s.balance.lock().unwrap() = None;
    Ok(())
}
#[tauri::command]
async fn set_touch(app: AppHandle, enabled: bool) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let message = if enabled {
            state.pad.enable(app.clone())?
        } else {
            state.pad.disable();
            "手柄已释放".into()
        };
        let _ = app.emit("pad-status", json!({"enabled":enabled,"message":message}));
        Ok(message)
    })
    .await
    .map_err(|_| "手柄任务异常".to_string())?
}
fn panel(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("panel") {
        let _ = w.show();
        let _ = w.set_focus();
        return Ok(());
    }
    WebviewWindowBuilder::new(
        app,
        "panel",
        WebviewUrl::App("index.html?window=panel".into()),
    )
    .title("WhaleTouch · 鲸抚")
    .inner_size(1080.0, 760.0)
    .min_inner_size(860.0, 640.0)
    .decorations(false)
    .transparent(true)
    .center()
    .build()
    .map_err(|_| "无法打开控制面板")?;
    Ok(())
}
fn show_pet(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("pet") {
        w.show().map_err(|_| "无法显示桌宠")?;
        let _ = app.emit("pet-visible", true);
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "pet", WebviewUrl::App("index.html?window=pet".into()))
        .title("WhaleTouch")
        .inner_size(360.0, 560.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .build()
        .map_err(|_| "无法重建桌宠窗口")?;
    Ok(())
}
fn hide_pet(app: &AppHandle) {
    app.state::<AppState>().pad.disable();
    let _ = app.emit(
        "pad-status",
        json!({"enabled":false,"message":"桌宠已隐藏，手柄已释放"}),
    );
    let _ = app.emit("pet-visible", false);
    if let Some(w) = app.get_webview_window("pet") {
        let _ = w.hide();
    }
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;
        if let Some(w) = handle.get_webview_window("pet") {
            if !w.is_visible().unwrap_or(true) {
                let _ = w.destroy();
            }
        }
    });
}
#[tauri::command]
fn window_action(app: AppHandle, action: String) -> Result<(), String> {
    match action.as_str() {
        "panel" => panel(&app),
        "hide" => {
            hide_pet(&app);
            Ok(())
        }
        "show" => show_pet(&app),
        "close_panel" => {
            if let Some(w) = app.get_webview_window("panel") {
                let _ = w.close();
            }
            Ok(())
        }
        "quit" => {
            app.state::<AppState>().pad.disable();
            app.exit(0);
            Ok(())
        }
        _ => Err("未知窗口操作".into()),
    }
}
#[tauri::command]
fn pet_action(app: AppHandle, action: String) {
    if action == "eat" {
        let state = app.state::<AppState>();
        let mut progress = state.progress.lock().unwrap();
        progress.feeds += 1;
        let _ = persist(state.dir.join("progress.json"), &*progress);
    }
    let _ = app.emit("pet-action", action);
}
#[tauri::command]
fn record_progress(app: AppHandle, seconds: u64, feed: bool) -> Result<(), String> {
    let s = app.state::<AppState>();
    let mut p = s.progress.lock().unwrap();
    p.companion_seconds += seconds.min(300);
    if feed {
        p.feeds += 1;
    }
    persist(s.dir.join("progress.json"), &*p)
}
#[tauri::command]
fn set_timer(
    app: AppHandle,
    operation: String,
    seconds: u64,
    label: String,
) -> Result<Value, String> {
    let s = app.state::<AppState>();
    let mut timer = s.timer.lock().unwrap();
    match operation.as_str() {
        "start" => {
            if seconds == 0 || seconds > 86400 {
                return Err("提醒时长应在 1 秒至 24 小时之间".into());
            }
            *timer = Some(Timer {
                id: epoch(),
                label: label.chars().take(80).collect(),
                ends: epoch() + seconds,
                remaining: seconds,
                paused: false,
            });
        }
        "pause" => {
            if let Some(t) = timer.as_mut() {
                t.remaining = t.ends.saturating_sub(epoch());
                t.paused = true;
            }
        }
        "resume" => {
            if let Some(t) = timer.as_mut() {
                t.ends = epoch() + t.remaining;
                t.paused = false;
            }
        }
        "cancel" => *timer = None,
        _ => return Err("未知计时操作".into()),
    }
    s.timer_changed.send_modify(|n| *n += 1);
    let v = json!(&*timer);
    let _ = app.emit("timer-changed", &v);
    Ok(v)
}
#[tauri::command]
async fn get_cubism_core(app: AppHandle) -> Result<String, String> {
    let s = app.state::<AppState>();
    let path = s.dir.join("live2dcubismcore.min.js");
    if let Ok(js) = tokio::fs::read_to_string(&path).await {
        return Ok(js);
    }
    let response = s
        .client
        .get("https://cubism.live2d.com/sdk-web/cubismcore/live2dcubismcore.min.js")
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|_| "首次启用 Live2D 需要联网取得官方运行时")?;
    if !response.status().is_success() {
        return Err("Live2D 官方运行时下载失败".into());
    }
    let js = response
        .text()
        .await
        .map_err(|_| "无法读取 Live2D 运行时")?;
    if !js.contains("Live2DCubismCore") {
        return Err("Live2D 运行时格式不兼容".into());
    }
    tokio::fs::write(path, &js)
        .await
        .map_err(|_| "无法缓存 Live2D 运行时")?;
    Ok(js)
}
#[tauri::command]
fn import_model(app: AppHandle, path: String) -> Result<String, String> {
    let source = PathBuf::from(path)
        .canonicalize()
        .map_err(|_| "模型路径不存在")?;
    if !source.to_string_lossy().ends_with(".model3.json") {
        return Err("请选择 .model3.json 文件".into());
    }
    let base = source.parent().ok_or("模型目录无效")?;
    let config: Value =
        serde_json::from_slice(&std::fs::read(&source).map_err(|_| "无法读取模型")?)
            .map_err(|_| "模型清单无效")?;
    let folder = app
        .state::<AppState>()
        .dir
        .join("models")
        .join(format!("import-{}", epoch()));
    std::fs::create_dir_all(&folder).map_err(|_| "无法创建模型目录")?;
    fn copy_refs(v: &Value, base: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
        match v {
            Value::String(file) => {
                if !file.contains('.') {
                    return Ok(());
                }
                let p = std::path::Path::new(file);
                if p.is_absolute()
                    || p.components()
                        .any(|c| matches!(c, std::path::Component::ParentDir))
                {
                    return Err("模型包含越界资源路径".into());
                }
                let from = base
                    .join(p)
                    .canonicalize()
                    .map_err(|_| format!("缺少模型资源：{file}"))?;
                if !from.starts_with(base) {
                    return Err("模型资源不能指向外部目录".into());
                }
                let to = dest.join(p);
                if let Some(parent) = to.parent() {
                    std::fs::create_dir_all(parent).map_err(|_| "无法创建资源目录")?;
                }
                std::fs::copy(from, to).map_err(|_| "无法复制模型资源")?;
            }
            Value::Array(a) => {
                for x in a {
                    copy_refs(x, base, dest)?
                }
            }
            Value::Object(o) => {
                for (k, x) in o {
                    if [
                        "Moc",
                        "Textures",
                        "Physics",
                        "Pose",
                        "DisplayInfo",
                        "File",
                        "Motions",
                        "Expressions",
                    ]
                    .contains(&k.as_str())
                        || x.is_object()
                        || x.is_array()
                    {
                        copy_refs(x, base, dest)?
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    copy_refs(&config["FileReferences"], base, &folder)?;
    let to = folder.join(source.file_name().unwrap());
    std::fs::copy(&source, &to).map_err(|_| "无法复制模型清单")?;
    Ok(to.to_string_lossy().into_owned())
}
fn main() {
    tauri::Builder::default()
        .setup(|app|{
            let dir=app.path().app_data_dir()?;std::fs::create_dir_all(&dir)?;
            let mut settings:Settings=read(dir.join("settings.json"));settings.pad.mapping.chibi=settings.character=="chibi";let pad=pad::Controller::default();*pad.options.lock().unwrap()=settings.pad;
            let (cancel,_)=tokio::sync::watch::channel(0);let (timer_changed,_)=tokio::sync::watch::channel(0);
            app.manage(AppState{progress:Mutex::new(read(dir.join("progress.json"))),notices:Mutex::new(read(dir.join("notices.json"))),dir,client:reqwest::Client::builder().user_agent("WhaleTouch/0.1").build()?,settings:Mutex::new(settings),pad,balance:Mutex::new(None),chat_cancel:cancel,timer:Mutex::new(None),timer_changed});
            use tauri::menu::{Menu,MenuItem};
            let show=MenuItem::with_id(app,"show","显示桌宠",true,None::<&str>)?;
            let controls=MenuItem::with_id(app,"panel","控制面板",true,None::<&str>)?;
            let release=MenuItem::with_id(app,"release","关闭触摸 / 释放手柄",true,None::<&str>)?;
            let hide=MenuItem::with_id(app,"hide","隐藏并释放手柄",true,None::<&str>)?;
            let quit=MenuItem::with_id(app,"quit","退出 WhaleTouch",true,None::<&str>)?;
            let menu=Menu::with_items(app,&[&show,&controls,&release,&hide,&quit])?;
            tauri::tray::TrayIconBuilder::new().icon(tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))?).tooltip("WhaleTouch · 鲸抚").menu(&menu).on_menu_event(|app,event|{
                match event.id.as_ref(){"show"=>{let _=show_pet(app);},"panel"=>{let _=panel(app);},"release"=>{app.state::<AppState>().pad.disable();let _=app.emit("pad-status",json!({"enabled":false,"message":"手柄已释放"}));},"hide"=>hide_pet(app),"quit"=>{app.state::<AppState>().pad.disable();app.exit(0)},_=>{}}
            }).build(app)?;
            show_pet(app.handle())?;
            // First launch opens controls. Closing them destroys the WebView.
            if !app.state::<AppState>().dir.join("settings.json").exists(){panel(app.handle())?;}
            let handle=app.handle().clone();let mut changes=app.state::<AppState>().timer_changed.subscribe();
            tauri::async_runtime::spawn(async move{loop{
                let timer=handle.state::<AppState>().timer.lock().unwrap().clone();
                if let Some(t)=timer.filter(|t|!t.paused){
                    tokio::select!{_=changes.changed()=>{},_=tokio::time::sleep(Duration::from_secs(t.ends.saturating_sub(epoch())))=>{
                        let s=handle.state::<AppState>();let mut current=s.timer.lock().unwrap();
                        if current.as_ref().is_some_and(|x|x.id==t.id&&!x.paused&&x.ends<=epoch()){
                            *current=None;let n=Notice{id:t.id,text:t.label,at:epoch()};
                            let mut notices=s.notices.lock().unwrap();notices.push(n.clone());if notices.len()>30{notices.remove(0);}let _=persist(s.dir.join("notices.json"),&*notices);
                            let _=handle.emit("timer-done",&n);
                        }
                    }}
                }else{let _=changes.changed().await;}
            }});
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "pet" {
                    api.prevent_close();
                    hide_pet(window.app_handle());
                } else if window.label() == "panel" {
                    window.app_handle().state::<AppState>().chat_cancel.send_modify(|n| *n += 1);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![snapshot,save_settings,save_key,set_touch,window_action,pet_action,record_progress,set_timer,get_cubism_core,import_model,api::get_balance,api::list_models,api::start_chat,api::cancel_chat])
        .build(tauri::generate_context!()).expect("WhaleTouch 启动失败")
        .run(|app,event|{if let tauri::RunEvent::Exit=event{app.state::<AppState>().pad.disable();}});
}
