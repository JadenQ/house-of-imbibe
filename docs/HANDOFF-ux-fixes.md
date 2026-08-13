# House of Imbibe — Handoff：修复 UX 审查发现的 bugs

> 上一 session 完成了 4 UX 修复 + PixelLab 升级（地图256/角色48/walk动画）+ 管理员密钥制 + 中文 README，并跑了一轮 8-agent UX 审查。**本 session 聚焦：修复审查发现的 2 critical + 6 major + 10 minor bugs。**

## 项目速览

- 路径：`/Users/jaden/Documents/house-of-imbibe`
- 栈：Rust + Axum 0.8 + SQLite(sqlx) 单二进制 + Phaser 4 + TS + Vite，逻辑分辨率 240×160 整数缩放，移动端横屏优先
- 约束权威：`CLAUDE.md`（三条禁令：HTTP 路径不等生成 / 聊天不落库 / scene 无 avatar kind 分支；锁定决策见新增章节）
- 代码全貌：`README.md`（中文版，最新）
- 运行：`source .env && cargo run`（:8080，前台跑——后台常驻 task 会被环境清理 kill）；浏览器 http://localhost:8080；注册填 `i am jaden` 当 admin
- 编译验证：`cargo check --all-targets && cd web && npx tsc --noEmit && npm run build && cargo test --all-targets`

## 当前状态

- working tree clean，所有改动已 commit：
  - `d62c3e9` docs: README 中文版
  - `c84704b` admin user info（管理员密钥制）
  - `1ef5b80` PixelLab 地图256 + 角色48 + 4方向walk动画
  - `8d0f97b` 4 UX fixes + admin/menu/walkable/accessories
  - `4965197` D1 地图背景图生成

## 下一任务：修 audit bugs

**详细审查报告**：`docs/ux-audit-2026-08-13.md`（291 行，含全部 issues 的文件:行号+根因+修复建议 + 50 步浏览器自测指引）。下面是修复优先级摘要，细节查报告。

### 🔴 Critical（必修，已代码确认属实）

1. **远端玩家永久不可见 + 资源洪泛** — `web/src/scene/BarScene.ts:300,307,313`
   - 根因：sheetCache 键不一致。`:300` `has(pv.avatarHash)`（裸 hash）、`:307` `set(av_<hash>, ...)`、`:313` `get(pv.avatarHash)`（裸 hash）→ get 永远 undefined → 远端 sprite 永不 `setVisible(true)` + 每完成一轮重入 `prepareCharacterSheet`（generated 反复下载 4方向×3帧 PNG）→ 多人房间标签页卡死/内存耗尽
   - 修复：三处键统一。推荐都用 `texKey = av_${pv.avatarHash}`（has(texKey) / set(texKey) / get(texKey)）
   - 验证：两浏览器互见远端 sprite 正常贴图 + DevTools Network 无重复 /api/assets 下载

2. **封禁对已登录用户完全无效** — `src/lib.rs:114` current_user + admin_ban
   - 根因：`current_user` 的 SELECT 只取 `(id,username,is_admin)` 不读 `banned` 列 → 所有非 login 端点 + WS 放行被封用户；`admin_ban` 只 `UPDATE banned=1` 不清 `sessions` → 30 天 cookie 仍有效；被封 admin 的 `is_admin` 不变 → 仍能访问 `/api/admin/*`；无邮箱无 IP 门禁可换名重注册绕过
   - 修复：`current_user` 读 `banned`，banned=true 返回 401/403；`admin_ban` 同时 `DELETE FROM sessions WHERE user_id=?` 清 cookie；考虑封禁 admin 时拒绝或降级
   - 验证：curl 封禁后用旧 cookie 访问 /api/me → 应 401

### 🟠 Major（6，详见 audit 报告）

1. **HUD 换装杀后台轮询** — `web/src/main.ts:82` `showAvatarCreate(avatar).then(reload)`；生成/文字分支提交即 resolve 触发 reload 销毁 toast+轮询。修复：生成路径不 reload，保留 `pollAvatarInBackground`（仅 modular 保存路径 reload）
2. **D4 generated 配饰无 UI 路径** — `avatarBuilder` 非 modular 传 null + `put_avatar` 硬写 kind=modular + `avatar_equip` 要真实 asset_id（preset 404）。修复：给 generated 加配饰 UI，或 `put_avatar` 接受 generated+equipped 透传
3. **localStorage 串设备覆盖服务端** — `web/src/ui/avatarBuilder.ts:38-50` `loadModularLocal` 用旧本地值覆盖服务端新值（注释过时，后端已持久化样式）。修复：移除或弱化 `loadModularLocal`（后端 put_avatar 已存 hairStyle/equipped）
4. **聊天侧栏 50 条后冻结** — `web/src/game-state/room.ts:130` shift 封顶 50 + `BarScene.ts:340` `length !== lastChatLen` 恒 false → setChat 不再调用。修复：用版本号或 `length + lastMsgId` 判断刷新
5. **admin walkable 网格编辑对碰撞无效** — `BarScene.ts:169` solidAt 用硬编码 BAR_MAP + `src/grid.rs` 用静态 bar.json，都不读 DB `maps.walkable`。修复：`solidAt` 读 `maps.walkable`（DB），服务端 `grid.rs` 同步读 DB
6. **管理台/酒单打开键盘泄漏到 Phaser** — `web/src/ui/admin.ts` 表单无 stopPropagation，打字时 WASD 移动角色。修复：admin 输入 keydown stopPropagation，或 `BarScene.update` 检查 adminOpen/menuOpen 早退

### 🟡 Minor（10，详见 audit 报告）

cookie 缺 Secure + session 永不过期 · register 错误一律误报 username taken · Unicode 用户名仿冒 · boot 第二次 me() 无 try/catch · 装饰 PNG 加载失败每帧重试 · 酒单打开角色可走动 · 时钟未扣单程延迟 · scene_changed/kicked 未实现 · done job「应用」一律 reload 最新 · 重生成背景超 4min 误报超时。

## 环境注意

- `.env` 有 `PIXELLAB_API_KEY`/`MINIMAX_API_KEY`（真实 key，已脱敏不入本文档；`.env` 在 .gitignore，但 git 历史里早期 README 曾硬编码真实 key——如 key 仍在用建议去控制台轮换）
- **Bash 分类器间歇不可用**（glm-5.2/模型安全判定 "temporarily unavailable"），影响 `cargo`/`curl`/`git` 等需安全判定的命令；只读 Read/Grep 不受影响。**Workflow 用 sonnet agent 可避开**（sonnet 分类器稳定）
- 后台常驻 server（`run_in_background` 的 cargo run）会被环境清理 kill——用前台 `source .env && cargo run` 或用户 `!` 自启
- 数据库 `data/hoi.db`（SQLite WAL）；现有测试用户 founder(admin)/normie/jaden2/pop 等

## Suggested skills

- **verify**：每修一个 critical/major 后端到端验证（跑 affected flow，不只 tsc）——这是修复可信度的关键
- **code-review**：全部修完后 `/code-review` review 修复质量（correctness + simplification）
- **diagnosing-bugs**：若 critical #1（sheetCache）或 #2（封禁）修复后行为仍异常，用诊断循环深挖
- **tdd**：给 critical 加回归测试——sheetCache 键一致性、封禁后 cookie 失效、walkable 接碰撞（防回归）

## 修复建议顺序

1. Critical #1（sheetCache 键统一）— 1 行级修复，影响最大（多人可见）
2. Critical #2（封禁 current_user 读 banned + 清 sessions）
3. Major #6（键盘泄漏 stopPropagation）+ #4（聊天冻结 length 判断）— 小改快见效
4. Major #1（换装杀轮询）+ #3（localStorage 串设备）— 形象流
5. Major #5（walkable 接碰撞）— 跨前后端，较大
6. Major #2（generated 配饰 UI）— 半成品补全，最大
7. Minor 批量扫尾

修完跑 verify + code-review，再 commit。
