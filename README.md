# WhaleTouch · 鲸抚

用 DualSense 抚摸 DeepSeek 鲸鱼娘。全身分区、双形态、官方 API 聊天与余额查询，iOS 风格轻玻璃界面。

**Windows 桌面应用 · Tauri + Rust · 手柄默认不接管 · 无操作静态休眠**

## 使用

从仓库 **Actions → Windows package → Run workflow** 构建。在成功运行的 Artifacts 中下载 Windows 包；便携版解压后运行 `whaletouch.exe`，也可用 NSIS 安装包。当前为未签名的 0.1 开发版本。安装与硬件实测状态见 [验证记录](docs/VALIDATION.md)。

1. 第一次打开会显示桌宠和控制面板；以后右键桌宠或托盘打开面板。
2. 在「习惯与设置」选择 Q 版或成女版；默认使用全身图集。可选 Live2D 是桌前半身模型，也可导入自己的完整 `.model3.json` 模型。
3. **DualSense 竖着用**：以手柄正面看，触摸板最左端是头，最右端是脚，中间按身体比例分区。
4. 接入 USB 形态的 DualSense，手动开启「触摸模式」。**不开启就不访问手柄**；关闭、隐藏和退出会停止反馈、释放句柄。暂不支持蓝牙直连。
5. 在设置中保存 DeepSeek 官方 API Key，按需刷新模型列表。聊天、余额使用官方接口，本地抚摸不需要密钥。

控制面板中的分区条可以用鼠标试摸，不会连接手柄。桌宠鼠标拖动可提起，右键打开面板；按 Shift 拖动可连续试摸。穿透模式可从控制面板恢复。

## 已实现

- 成女／Q 版图集、可选 Live2D 与本地模型导入，待机、全身触摸、戳脸、提起、落地、走路、投喂、睡醒、聊天和专注。
- 原生 USB HID 读取、竖向映射、分区视觉反馈、速度相关震动、灯条与手柄释放。
- 官方流式聊天及取消、模型查询、余额明细、本应用 token 累计和每日 UTC 用量。
- 投喂、陪伴时长、25/5 分钟番茄钟、一次性提醒、勿扰。首版同时保留一个计时任务，新建会提示替换。
- 30 秒无操作停动画，隐藏后销毁渲染窗；设置与聊天面板关闭即销毁。

## 轻量化边界

没有常驻 Python、Node、本地 HTTP 服务或本地大模型；仅开启触摸时打开 HID。关闭模式不枚举、不读取、不发送输出，也不后台重连。

图片动作按需采样，Live2D/Pixi 代码仅启用时动态加载。静态休眠不持续刷新 Canvas；隐藏立即释放手柄和图片／纹理，30 秒后销毁桌宠 WebView。窗口命中检查只在可见时运行。

玻璃感由半透明色、亮边和静态高光构成，不使用持续模糊或折射着色器。Tauri 使用系统 WebView2，**不等于单进程或零内存**。实际资源占用需统计整个应用进程树，不能用安装包大小替代。

## 官方 API 与数据

后端仅直连 `https://api.deepseek.com`，使用 `/chat/completions`、`/models`、`/user/balance`。没有中转服务；密钥使用当前 Windows 用户的 DPAPI 加密保存在应用数据目录，不进入前端持久存储或日志。

余额在面板打开时按需查询，60 秒内复用缓存，关闭后不轮询。币种分别显示，余额不是官方完整账单。token 仅累计本应用收到的官方 usage；取消或断流未收到 usage 时不虚构用量。

触摸坐标不发送云端。可选互动台词默认关闭，开启后仅在控制面板打开时发送简短摘要，至少冷却 20 秒。聊天历史保存在本机，可从聊天页清空；设置、陪伴、提醒与用量保存在 `%APPDATA%/io.amadeus.whaletouch/`。

## 开发

前置：Node.js 22+、Rust stable；Windows 构建还需要 MSVC C++ Build Tools 和 WebView2 Runtime。

```sh
npm ci
npm run dev            # 浏览器 UI / 鼠标演示，不连接手柄、不存密钥、不调用 API
npm run desktop        # Windows 桌面开发
cargo test -p whaletouch-core  # 必要 Core 测试
npm run package        # Windows 构建
```

只做受影响的 Core 测试与最小功能验证，不铺全量 E2E、截图回归、长压测或素材哈希清单。详见 [开发约束](docs/DEVELOPMENT.md)。

## 美术与许可

角色原作 **上善无形**，DeepSeek 女仆二次设计 **ZipZipPipe**；Q 版素材 **YunYueSama**，Live2D 模型 **氵六青**。成女全身动作图集为基于 ZipZipPipe 原稿的 AI 辅助改编。协议参考 **DukeCCY/dualsense-pad**，动作组织参考 **VPet**。

代码 MIT；鲸鱼娘角色及适用美术 **CC BY-NC-SA 4.0**，Q 版新增贡献另保留其自定义署名许可。**代码开源不等于美术允许商用**。完整来源、修改说明与许可证见 [NOTICE.md](NOTICE.md) 和 [licenses/](licenses/)。
