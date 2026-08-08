# 像素社交聊天空间 · 调研文档索引

> 生成时间：2026-07-27（最后更新 2026-08-01）
> 项目定位：类口袋妖怪-绿宝石画风的 Web 像素马赛克社交聊天空间；一人 + AI 辅助开发。

## 📐 先读这个
- [**CONVENTIONS.md**](./CONVENTIONS.md) — **文档编写规范**：三层结构（决策/调研/事实）、
  事实标注（✅已核实 / 📄官方声明 / ⚠️未核实 / ❌已证伪）、第三方 API 记录清单
  - **关键规则**：`research/` 下的结论**不可直接当事实引用**。要写进代码的接口细节，
    必须先落到 `reference/` 并附核实命令

## ⭐ 实际采用方案（v2）
- [**pixel-mosaic-game-workflow-v2-rust.md**](./pixel-mosaic-game-workflow-v2-rust.md) — **Rust 后端 + 自建注册系统 + 单 VPS 部署**（当前采用版本）
  - 前端保持不变：Phaser 4 + TypeScript + Vite
  - 后端换：Supabase → Axum + SQLite + axum-login
  - 部署换：Vercel/PartyKit → Hetzner VPS + Caddy + systemd
  - 成本：€4.5/月封顶

## 📗 事实层（已核实的第三方 API）
- [**reference/pixellab-api.md**](./reference/pixellab-api.md) — **PixelLab API 已核实事实**
  （核实日期 2026-08-01，依据 OpenAPI spec + MCP 实测 + 官方定价页）
  - 端点/参数/尺寸硬上限/精确价格/异步轮询语义/CORS/65 个 MCP 工具
  - **三条最重要的结论**：① 生成是**异步**的，端到端 5–9 分钟，无单端点一步到位；
    ② **MCP 只能开发期用**，用户侧必须自建后端代理（否则 token 泄露）；
    ③ 尺寸有硬上限（角色 ≤128/256px），**不存在 256×384 这类输出**

## 📘 参考方案（v1，供对照）
- [pixel-mosaic-game-workflow.md](./pixel-mosaic-game-workflow.md) — v1 SaaS 路线（Supabase + PartyKit），保留作为对照
  - v1 → v2 迁移清单在 v2 报告 §四

## 📚 详细维度调研

> ⚠️ 调研层会过期。涉及 PixelLab 的部分已在 2026-08-01 就地标注 ❌/⚠️，以事实层为准。

### 前端 / 素材 / AI（v1，仍适用）
0. [物理世界 → 像素游戏素材生成](./物理世界转像素游戏素材-调研报告.md) — ⚠️ 部分过期，PixelLab 参数/MCP 结论已更正
1. [AI 像素生成工具与模型](./research/01-ai-pixel-generation.md) — Retro Diffusion / PixelLab / fal.ai / Civitai LoRA（⚠️ Retro Diffusion 全段未实测）
2. [照片 → 像素头像转换管线](./research/02-photo-to-pixel-pipeline.md) — InstantID / IP-Adapter / SDXL Pixel LoRA
3. [角色自定义创建器](./research/03-avatar-character-creator.md) — LPC / CharaKit / Mana Seed
4. [Web 2D 游戏引擎选型](./research/04-web-game-engine.md) — Phaser 4 / RPG.js / Kaplay
5. [~~实时多人后端~~](./research/05-realtime-multiplayer.md) — ⚠️ v1 版本，v2 已换成自建 Rust WS，参考 research/10
6. [瓦片地图设计](./research/06-tilemap-level-design.md) — Tiled / LDtk / Ninja Adventure / Kenney
7. [AI 编码开发工作流](./research/07-ai-dev-workflow.md) — Claude Code / Cursor / Kimi K2 / MCP / Skills
8. [动画/音效/字体/后处理](./research/08-animation-audio-assets.md) — Aseprite / Suno / jsfxr / Fusion Pixel / CRT

### 后端 · Rust（v2 新增）
9. [Rust Web 框架选型](./research/09-rust-web-framework.md) — Axum / Actix / Loco / Salvo / Poem
10. [Rust 实时多人方案](./research/10-rust-realtime-multiplayer.md) — Axum WS / Naia / Lightyear / Renet
11. [Auth + 数据库层](./research/11-rust-auth-database.md) — axum-login / argon2 / sqlx / SQLite 完整流程
12. [资产存储](./research/12-rust-storage-assets.md) — object_store / R2 / 本地磁盘 / 图像处理
13. [部署与运维](./research/13-rust-deploy-devops.md) — Hetzner / Fly.io / Shuttle / Caddy / systemd

## 🚀 立即行动（摘自 v2 §八）
详见 v2 报告最后一节。三步骨架：
1. `cargo new` + Axum + sqlx + tower-sessions 起项目骨架
2. 打通注册/登录 + Phaser 前端登陆页
3. WebSocket 双人 demo（浏览器 `new WebSocket()` 连自己的 Rust 服务器）

## 🔑 一句话技术栈（v2）
**前端**：Phaser 4 + TS + Vite
**后端**：Axum 0.8 + SQLite (WAL) + sqlx 0.8 + axum-login + argon2 + lettre/Resend
**实时**：Axum WS + tokio::broadcast + DashMap
**存储**：object_store（本地 → R2 零改动）
**AI 素材**：fal.ai（同步秒级，头像）/ PixelLab REST v2（异步分钟级，行走 sprite）/ LPC / CharaKit / Ninja Adventure
**部署**：Hetzner CX22 + Caddy + systemd（€4.5/月）
**AI 编码**：Claude Code + Cursor + rust-analyzer MCP + PixelLab MCP（出素材用）

## 修订记录

- **2026-08-01**：新增 `CONVENTIONS.md`（文档规范）与 `reference/` 事实层；
  新增 `reference/pixellab-api.md`；标注 4 份调研文档为部分过期。
  起因：发现调研文档中存在编造的 API 参数（`block_size`）、错误的 MCP 结论
  （"无官方 MCP"、工具数 7 vs 实际 65）、不可能的输出尺寸（256×384）
  以及不存在的功能（"嵌入式 widget"）。
