# 来源、许可与修改

本项目为非官方、免费、非商业的 DeepSeek 鲸鱼娘同人桌宠，与 DeepSeek 及下述作者无官方合作关系。

## 代码

WhaleTouch 新增代码：MIT，Mozart Amadeus。

USB 报文解析、输出字段及竖向坐标映射改编自 [DukeCCY/dualsense-pad](https://github.com/DukeCCY/dualsense-pad)（MIT）。本项目用 Rust 重写 Windows 设备接管、交互与反馈调度，未引入其 Python HTTP 服务。原许可见 `licenses/dualsense-pad.txt`。

动作类型及开始／循环／结束的组织方式参考 [LorisYounger/VPet](https://github.com/LorisYounger/VPet)，未复制其人物图片或动画代码。

## 角色版权链

- **上善无形（上善）**：原创鲸鱼娘「溟月」；https://space.bilibili.com/4456176
- **ZipZipPipe**：DeepSeek 女仆鲸鱼娘二次设计；https://space.bilibili.com/4168597 ，https://www.pixiv.net/artworks/148186519
- 作者关于形象的声明：https://www.bilibili.com/opus/1247156596114456581

形象适用 **CC BY-NC-SA 4.0**（署名、非商业、相同方式共享）：https://creativecommons.org/licenses/by-nc-sa/4.0/ 。代码的 MIT 不覆盖美术与角色。未经相关权利人分别授权，不得将这些角色素材用于商业目的。

## 随包图片和模型

| 路径 | 来源与修改 | 许可 |
| --- | --- | --- |
| `public/reference/zipzippipe-adult.jpg` | ZipZipPipe 原始正常比例立绘，经 [shuaizenge/ai-girls](https://github.com/shuaizenge/ai-girls/tree/main/characters/deepseek) 取得；原图未修改 | 角色许可链 CC BY-NC-SA 4.0 |
| `public/characters/adult/actions.png` | WhaleTouch 基于上图使用 image_gen 制作的 AI 辅助二创：4×3 全身姿态图集，补充抚摸、进食、阅读、睡眠、走路、提起与落地姿态；不是 ZipZipPipe 原作或原作者新增动作 | 新增改编贡献 CC BY-NC-SA 4.0，保留原角色署名 |
| `public/characters/chibi/actions.png`、`idle.png` | YunYueSama / https://github.com/YunYueSama/codex-deepseek-pet ，来自 `assets/whale/actions.png`、`idle-front.png`；图片未修改，运行时按格采样，新增动作映射 | 新增贡献遵守 `licenses/YunYueSama.txt` 自定义署名许可；底层角色仍遵守其原许可 |
| `public/characters/live2d/**` | 模型：氵六青，https://space.bilibili.com/11272072 ；从 [A8Chann/dsh-pet-live2d](https://github.com/A8Chann/dsh-pet-live2d) 的 `dsh-live2d-pet/pets/ds-whale-girl/` 取得；随附清单、模型、贴图、表情和动作未修改，WhaleTouch 新增参数驱动 | CC BY-NC-SA 4.0；随包 `LICENSE` 和 `licenses/A8Chann-NOTICE.md` |

这套 Live2D 为桌前半身模型，不是完整全身成女模型；默认全身图集负责完整头脚展示。序列帧采用关键姿态配合轻微程序过渡，不代表每秒 60 张独立原画。

## 运行时与第三方依赖

- Live2D Cubism Core 是 Live2D Inc. 的专有运行时，不随仓库分发。用户启用可选 Live2D 模式时从官方地址取得并缓存在本机；使用应遵守其官方条款：https://www.live2d.com/eula/live2d-proprietary-software-license-agreement_en.html 。许可获得方式以官方当前条款为准，本项目不额外授予其权利。
- pixi.js、pixi-live2d-display、Tauri、Rust 及 npm/Cargo 依赖遵守各自许可。依赖锁文件保留版本，未对依赖许可证重新授权。
- 软件图标由 WhaleTouch 使用几何图形绘制，适用 MIT。

记录日期：2026-11-07。若来源或署名存在错误，请在本仓库提出 Issue 以便修正。
