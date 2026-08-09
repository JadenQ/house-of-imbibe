---
id: 10
title: "Slice plan supplement — 地图生成 + 人物生成澄清"
status: decided
labels: [plan, locked]
feature: house-of-imbibe
---

# 地图与人物生成方案补充（对照用户 2026-08-08 需求）

> 对照 PRD(#1) + dev-plan + `docs/reference/pixellab-api.md`(实测) + 切片 issue #3/#4/#5/#6/#7。
> 结论：人物生成方案基本明确（切片2/3/4），**地图「生成」是新需求**（当前方案=手绘静态 `.tmj`）。本 issue 补缺口 + 锁澄清 + 列待决策。

## 需求1：像素地图生成（图片/文字）+ 家具摆件 admin 增删

### 现状
- 家具/摆件 admin 增删：✅ 已有方案（PRD §Map&decorations + issue #7 切片6：`decorations` 表 + admin REST CRUD + WS `{decoration_added|removed}` live broadcast + 资产来自 PixelLab `create_map_object` 或 curated）。
- 地图本体生成：❌ 当前 = 手绘 Tiled `.tmj` 静态底图（PRD + issue #6 切片5）。**无「图片/文字 → 生成像素地图」**。

### 补充方案：地图三层架构（各自独立来源）
1. **视觉背景层（生成）**：文字 → `create-image-pixen`（≤512）或 图片 → `image-to-pixelart`（输入≤1280，输出≤320），产出一张像素背景图（240×160 整数倍）。可选 `create-tileset`(top-down 16/32) 生成可平铺瓦片。
2. **可走/碰撞网格层（admin 可编辑）**：tile 网格，每格 walkable/blocked；admin 在编辑模式涂格子（或从背景图派生初值后人工校正）。服务端 clamp 读这层。落 DB（与 decorations 同源 `maps`/`map_cells` 表）。
3. **家具装饰对象层（DB + admin CRUD）**：= 现有 decorations 方案（issue #7），资产来自 `create_map_object`。

> 关键认知：PixelLab 生成的扁平像素图 ≠ 可走/碰撞的 tilemap。所以「生成地图」= 生成背景图 + 独立的可走网格 + 独立装饰对象，三者解耦。

### 待决策
- **D1 地图生成路径**：A) 生成整张背景图（pixen/image-to-pixelart，~$0.007，便宜但不保证可平铺）vs B) 生成瓦片集（create-tileset，可平铺但需拼装）vs C) 背景+地面瓦片集。**建议 MVP=A**，地面拼贴后期增强。

## 需求2a：modular 预设形象（发型/发色/肤色/上下装/鞋）

### 现状：✅ 方案明确（issue #3 切片2，curated preset + HSL recolor，本切片无 PixelLab）。
### 补充：preset 部位分类显式化
slots = `body`(肤色) / `hair`(发色) / `top`(上衣) / `bottom`(下装) / `shoes`(鞋) / `accessory`(背/手，切片3)。
issue #3 原文只写 hair/outfit/accessory，本补充把 skin→body slot、上下装拆开、加 shoes slot。

## 需求2b：图片→像素形象（直转 / vision→text 回退）

### 现状：🟡 方向锁定（CLAUDE.md 切片4输入管线），两处不明确。
### 补充澄清
1. **主路径「image-to-pixelart 直转」产出的是单张像素图，非可走多方向角色**（pixellab-api.md §六明确警告：`image-to-pixelart` 只是"照片降采样成像素画"，出不来朝南站姿游戏 sprite）。完整角色仍需：
   `照片 → image-to-pixelart(得 south 像素参考) → create-character-v3(reference_image, south≤256×256, 旋转 8 方向) → animate-character(行走)`。
   CLAUDE.md「直转」应理解为「直转得 south 参考」，**后续 v3 旋转+animate 不可省**。
2. 回退（vision→text）用 `create-character-with-4-directions` 只出 4 方向，与 PRD canonical **8 方向**契约不一致 → 决策见 D2。

### 待决策
- **D2 generated 角色 4 vs 8 方向**：A) 回退用 `create-character-v3`(从零,8方向) 统一 8 方向（贵~$0.041+动画，契约一致）vs B) 接受 4 方向(south/north/east/west)，对角靠镜像/省略（便宜~50%，渲染器需支持 4-dir 降级）。**建议 B**（MVP 省 50% 成本，对角镜像常见 RPG 做法）。

## 需求2c：文字→像素形象
### 现状：✅ = 2b 回退路径独立入口（issue #5 `kind=avatar_generated` 支持 text）。无需补充。

## 约束：一个用户只能一次生成个人形象
### 现状：🟡 PRD/issues 未显式声明，但 schema `avatars.user_id PRIMARY KEY` 已天然限定每用户一行（modular/generated 二选一，upsert 覆盖）。
### 待决策
- **D3「一次」语义**：A) 同时只持有一个、可重新生成覆盖（当前 upsert 行为，重生成丢弃旧的）vs B) 真只能生成一次、不可改。**建议 A**（可覆盖，体验合理；B 是产品决策）。

## Accessories（手上物品 + 背饰）
### 现状：✅ 方案明确（issue #4 切片3，back/hand slot，runtime overlay 装到 modular）。
### 冲突
- PRD 规定 accessories **不能装配到 generated avatar**（non-interoperability）。用户需求把 accessories 列在「人物生成」通用项下，似期望 generated 也能拿/背 → 与 PRD 冲突。
### 待决策
- **D4 generated 角色是否允许 accessories**：A) 维持 PRD（generated 不可配，省事、契约简单）vs B) 放宽（generated 也支持 back/hand overlay，增复杂度）。**建议 A**（除非用户明确要 B）。

## 本期开发（决策无关的地基 = 切片3a + 禁令#1修复）

D1-D4 任一如何定，生成管线都依赖同一地基（已在并行开发，见 handoff §3 三个 🔴）：
- `AssetStore` trait + `LocalAssetStore` + `GET /api/assets/{key}`（**存 key 不存 URL**）
- `generation_jobs` 表（落库，重启不丢）
- `avatar_generate_submit` 改为：handler 仅校验+入队+返回 `job_id`（禁令#1）；后台 worker 认领 pending job → 跑管线 → 下载 PNG 进 AssetStore → `config_json` 存 key
- SQLite FK 启用 + 首用户 admin 竞态修复 + 测试

## 分期归属
| 需求 | 切片 / issue |
|---|---|
| 地图生成背景层 | 新，并入切片5 #6（加「背景层可由生成而来」AC） |
| 地图可走网格 admin 编辑 | 切片6 #7 扩展 |
| 家具摆件 admin 增删 | 切片6 #7 |
| modular 预设形象 | 切片2 #3 |
| 生成管线基建 | 切片3a #4 |
| 照片形象 + 文字形象 | 切片4 #5 |
| accessories | 切片3b #4（+切片4 装配） |
| admin 装饰增删 | 切片6 #7 |

## 决策清单（2026-08-09 用户已定稿，/grill-me 盘问后拍板）

- ✅ D1 地图生成路径 = **A 生成整张背景图**（pixen/image-to-pixelart，240×160，~$0.007；可走网格 admin 手标）
- ✅ D2 generated 方向 = **4 方向 + 行走动画**（`animate-character`），对角镜像，**非 8 方向**
- ✅ D3「一次生成」语义 = **A 可重新生成覆盖**（`avatars.user_id` PK + upsert，重生成丢旧的）
- ✅ D4 generated 配 accessories = **允许**（放宽 PRD non-interoperability，手物/背饰 overlay 装到 generated）
- ✅ 2a modular = **代码手绘部件扩展**（发型/上衣/下装/鞋子样式选项 + 捏脸 UI，**非 PNG preset**，无需美术）
- ✅ 2b 管线 = **中间档**：vision→text→4方向→`animate-character` 行走（**放弃「直转先」**：直转只出 south 单图不划算）
- ✅ 2c 文字入口 = **加**（POST 文字描述 → worker text 分支，复用 create-character-with-4-directions）

> generated avatar `config_json` 契约：`{kind:"generated", character_id, frames:{south:[key…],north:[…],west:[…],east:[…]}}`（每方向帧 key 数组，1=静站，3=行走）。

## 交互 UX + Admin 决策（2026-08-09 design-an-interface 盘问后定稿）

- ✅ **Admin = Design 2 独立管理台**（DOM console，is_admin 才显示「管理」入口，tabs:地图/成员/装饰；scene 保持只读；移动端优先）。详见 CLAUDE.md。
- ✅ **点单 = 只读看单**（menu 仅展示，不下单）。
- ✅ **异步形象 = 非阻塞 library**（生成中可先入场玩，job 后台轮询，完成通知/热替换；不再阻塞模态）。
- ✅ **modular 样式持久化**：`put_avatar` 必须完整存 hairStyle/topStyle/bottomStyle/shoeStyle/shoes（不剥离成 4 色）+ WS 快照透传。
