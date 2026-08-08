Note: glm-5.2[1M] (the safety classifier) was unavailable when reviewing this subagent's work. Please carefully verify the subagent's actions and output before acting on them.

# 实时多人同步与聊天后端 — 深入调研

## 1. PartyKit（Cloudflare Durable Objects 封装）
- **URL**：https://www.partykit.io/  ·  文档 https://docs.partykit.io/
- **用途**：基于 Cloudflare Workers + Durable Objects 的"房间即对象"编程模型。每个"party"就是一个 Durable Object，天生适合像素地图/聊天室这种"每房间一份权威状态"的场景。WebSocket、hibernation、broadcast 都是原生 API。
- **优点**：
  - 代码模型极简：写一个 `Server` 类实现 `onConnect / onMessage / broadcast()`，10~40 行就能跑起来（官方首页示例）。
  - 全球边缘部署，延迟一般 <100 ms；Durable Object 会自动"就近"贴到发起写入的用户。
  - PartyKit 已并入 Cloudflare（2024 年 4 月），个人平台**完全免费**（10 个项目、24 小时后 storage 清理）；商业用途只要 `--domain` 部署到自己的 CF 账号，PartyKit 平台费也是 0。
  - 与 Y.js / tldraw / XState 深度集成，Gather.town 类应用有大量参考实现。
- **缺点**：
  - CF Workers 有 CPU 时间/请求配额（Workers Free：10 万 DO 请求/日，13 000 GB-s 计算/日；Paid $5/月起）；高频 tick 循环需要注意 hibernation。
  - 没有 Colyseus 那种"内置 Schema 状态同步"，位置同步要自己写（其实很短）。
  - 生态相对 Colyseus/Nakama 小，游戏专用文档少。
- **成本/免费额度**：PartyKit 免费；底层 Cloudflare Workers Free 层足够几十上百 CCU 的原型；量上来后 Workers Paid $5/月 + Durable Objects 按用量（100 万请求/月免费，之后 $0.15/百万；WebSocket 消息以 20:1 比率计费）。
- **上手难度**：★☆☆☆☆（在这几家里最低）。`npm create partykit@latest` 一条命令出模板。
- **AI 友好度**：★★★★★。API 表面极小 + 官方文档结构清晰 + 已经有大量 GitHub 示例（cursor chat、Y.js 白板、聊天室），Claude Code / Cursor 一次基本能生成完整可运行代码。Sunil Pai 本人也是 AI 编程社区里最爱被引用的作者之一，训练数据覆盖好。

---

## 2. Colyseus（专业多人游戏服务器框架）
- **URL**：https://colyseus.io/  ·  文档 https://docs.colyseus.io/
- **用途**：Node.js 原生的房间制多人游戏框架，内置 `@colyseus/schema` 状态同步、matchmaking、interpolation client SDK（TS/Unity/Godot/Defold）。
- **优点**：
  - **状态同步是核心卖点**：在服务器上定义 `MapSchema<Player>`（含 x/y），客户端自动增量同步 —— 走位/血量/聊天几乎是"写数据结构"而不是"写协议"。
  - 开源、自托管完全免费，能跑在 Fly.io / Railway / 任意 VPS / Docker。
  - 官方 client SDK 覆盖 Web（TS）、Unity（C#）、Godot、Defold，未来扩展到 native 客户端零成本。
  - 官方 npm 模板 `npm create colyseus-app` + 大量教程 + Discord 活跃。
- **缺点**：
  - Colyseus Cloud **没有免费层**，最低 $15/月起（管理版）；不想付费必须自己在 Fly/Railway/Render 上部署，多一层 DevOps。
  - 单机 Node.js 进程，横向扩展需要自己配 presence + Redis / 多进程。
  - 客户端连接前需要 HTTP matchmaking 请求 —— 边缘部署没 PartyKit 那么"就近"。
- **成本**：自托管 = 一个 Fly.io shared-cpu-1x 约 $2–5/月 就能跑起来；Colyseus Cloud $15/月起，无 CCU 限制。
- **上手难度**：★★☆☆☆。文档非常好，但要理解 `Room` / `Schema` / `state.players` 三个概念。
- **AI 友好度**：★★★★☆。TypeScript 类型完善、Schema 模式约束强、模板文件结构固定，Claude/Cursor 生成"新增一个 room / 新加一个消息 handler"这类任务非常稳。缺点是 Schema 装饰器语法在旧模型知识中偶尔和 0.15/0.16/0.17 版本混淆，建议明确指定 `0.17`。

---

## 3. Liveblocks
- **URL**：https://liveblocks.io/  ·  定价 https://liveblocks.io/pricing
- **用途**：主打 SaaS 的"实时协作"基础设施（Presence、Storage CRDT、Comments、Yjs）。**不是**为游戏设计的，是为 Figma/Notion 类协作场景设计的。
- **优点**：React hooks 极其漂亮（`useOthers` / `useMyPresence`），5 分钟做出光标 + 头像栈；官方还发布了 MCP server（39 个工具）供 Cursor/Claude 直接操作房间。
- **缺点/致命点**：
  - **按"实时协作分钟数"计费**：2 人以上同房间时按 user-minute 计价（$0.002/分钟，Free 3 000 分钟/月）。像素小镇场景下用户一开就是几小时，**免费额度约 = 3 人各在线 33 小时/月就用完**，跑得起来但会突然变贵。
  - 面向"协作数据"而非"高频 tick"。位置 broadcast 是可行的但要克制频率（官方明确说不要用 Presence 做鼠标位移，改用 Broadcast）。
  - Free 计划页面强制显示 Liveblocks 品牌水印。
- **AI 友好度**：★★★★★（有官方 MCP，Claude 里一句"给房间加个头像栈"就能自动写代码）。
- **结论**：**不推荐做游戏后端**；可作为"聊天/评论/@mention"的锦上添花模块，但不是首选。

---

## 4. Supabase Realtime（Broadcast + Presence）
- **URL**：https://supabase.com/docs/guides/realtime  ·  Broadcast https://supabase.com/docs/guides/realtime/broadcast
- **用途**：Elixir/Phoenix 集群，提供 Broadcast（ephemeral 消息）+ Presence（CRDT 在线态）+ Postgres Changes。天然适合"多用户共享一个 channel 收发 JSON"。
- **优点**：
  - 已经在用 Supabase 做 Auth/Storage/DB？那 Realtime 是零额外配置的红利，`supabase.channel('map-1').send({...})` 就完事。
  - Presence 的 join/leave/sync 事件对"谁在线上"非常好用。
  - 官方 demo multiplayer.dev 就是光标 + 聊天，跟你的场景高度对齐。
  - 免费额度慷慨：2M Realtime 消息/月 + 200 并发连接 in Free tier。
- **缺点**：
  - **没有服务端权威逻辑**：Broadcast 是纯 P2P 中继，不校验消息。作弊/防碰撞/NPC AI 得自己写额外的 Edge Function。
  - Broadcast 频率打高（>10Hz 位置）在免费额度下 200 连接容易触限。
  - Presence 官方警告不要拿来做每帧位置。
- **AI 友好度**：★★★★☆。API 极简，Claude Code 熟悉 Supabase SDK，一次能出。
- **结论**：**最适合"以聊天 + 房间为核心，走位低频（10Hz 以内）"的原型**；如果你已经打算用 Supabase 做用户/头像存储，这条链路最省事。

---

## 5. Nakama（Heroic Labs）
- **URL**：https://heroiclabs.com/  ·  GitHub https://github.com/heroiclabs/nakama
- **用途**：开源游戏后端全家桶：Auth、社交、Chat、matchmaker、实时/回合制多人、leaderboard、purchase 验证。用 Go 写服务端逻辑（也支持 Lua/TS）。
- **优点**：功能最"全"；一个 Docker Compose 起 Nakama+CockroachDB 就有账号、好友、群聊。
- **缺点**：
  - **重**。要跑一个数据库 + Nakama 服务，本地开发内存占用可观；配置比 PartyKit 一个 `.ts` 文件复杂 20 倍。
  - Heroic Cloud **没有真正的免费层**，最低起价按 CPU/RAM 计费；自托管你要管 VPS + Postgres。
  - AI 生成代码相对差：Nakama 的 TS runtime API 曲面很大且版本变化多，Claude/Cursor 经常写错模块名或 hook 签名。
- **AI 友好度**：★★☆☆☆。
- **结论**：单人开发者原型阶段不划算，是个 overkill。

---

## 6. Photon Engine（Fusion / Quantum / Realtime）
- **URL**：https://www.photonengine.com/
- **用途**：Unity/Unreal 生态里的老牌 SaaS 网络方案。有 20 CCU 免费。
- **缺点**：主要面向 Unity/Unreal 客户端；Web 前端支持有但生态弱；不开源。**AI 友好度低**（Claude 对 Photon 的 API 掌握远不如 Colyseus/PartyKit）。**不推荐**用于 Web 优先的像素镇。

---

## 7. Socket.io + 自建 Node
- **URL**：https://socket.io/
- **优点**：Claude/Cursor 对 Socket.io 熟到不能再熟，任意 tutorial 都能生成。极其灵活。
- **缺点**：一切"房间制/状态同步/断线重连/横向扩展/部署 SSL"都得自己搭。原型阶段每一个都是坑。
- **结论**：**只有在已经想深度控制协议**（例如做 SLG 或格斗）时才值得。像素小镇不推荐。

---

## 8. Cloudflare Workers + Durable Objects 手撸
- **URL**：https://developers.cloudflare.com/durable-objects/
- 与 PartyKit 相同底层但没糖衣。所有 PartyKit 帮你抽走的（partysocket 客户端、`onConnect/onMessage` 生命周期、CLI `partykit dev`）都得手写。除非有特殊定制，否则**直接用 PartyKit** 就是这条路的最佳走法。

---

## 关键维度对比

| 方案 | 延迟 | 免费/入门成本 | 部署复杂度 | 状态同步内置 | AI 生成代码难度 |
|---|---|---|---|---|---|
| **PartyKit** | 边缘 <100ms | 完全免费（个人）或 CF $5/月 | 一条 `npx partykit deploy` | 无（自写，但极简） | 极低 |
| **Colyseus (自托管)** | 视 VPS 位置，50–150ms | $2–5/月 VPS | 中：Docker + Fly | 有（Schema） | 低 |
| **Colyseus Cloud** | 32 个 region | **$15/月起，无免费** | 极低（CLI 一键） | 有 | 低 |
| **Supabase Realtime** | 50–200ms | 免费 200 连接 / 2M msg | 零（SaaS） | 无（自写） | 低 |
| **Liveblocks** | 50–150ms | Free 3000 分钟/月（易超） | 零（SaaS） | 有（CRDT，非游戏专用） | 极低（有 MCP） |
| **Nakama** | 视自托管 | 免费自托管 / Heroic Cloud 昂贵 | 高 | 有 | 中高 |
| **Photon** | 好（Unity 生态） | 20 CCU 免费 | 低 | 有 | 高（Web 弱） |
| **Socket.io 自建** | 视部署 | VPS $2–5/月 | 高 | 无（全部自写） | 低（代码多） |

---

## 首选推荐

**主方案：PartyKit（房间 + 走位 + 聊天）＋ Supabase（Auth + 用户资料/头像/存图）**

理由：
1. **单人 vibe coding 友好度最高**：PartyKit 的编程模型就是一个 `Server` 类 + `onMessage / broadcast`，Claude Code 一句 prompt（"给我写一个 partykit server：维护一个 `players: Record<id, {x,y,skin,name}>`，广播 `move / chat / join / leave`"）就能生成完整可运行代码，40 行以内。
2. **免费到远超原型阶段**：PartyKit 平台费永远为 0，Cloudflare Workers Free 层足够跑几十~上百并发；跑爆前几乎不花钱。
3. **零 DevOps**：`npx partykit deploy` 一条命令上线，边缘部署天然低延迟，用户直接分享 URL 就能进入 —— 完全符合"浏览器可访问，便于分享"。
4. **Supabase 补齐后端存储**：像素小镇需要"登录 + 保存自定义头像 + 上传原图给 AI 转换"，Supabase 的 Auth（含 Google/GitHub OAuth）+ Storage（图片桶）+ Postgres（角色档案）刚好填这个洞；Realtime 也在里面，聊天/低频状态可以选择走 Supabase Broadcast 而不是 PartyKit，一次连接搞定。
5. **两者都是 Claude/Cursor 一等公民**：PartyKit 官方就是 AI 时代产物、Supabase 有官方 MCP + Claude 训练数据里样例极多，几乎不会写错。

**回退方案（如果之后需要"服务端权威 + 复杂游戏逻辑 + Unity 客户端"）**：迁移到自托管 **Colyseus 0.17 on Fly.io**（$2–5/月），因为它的 Schema 状态同步在"人物走位 + 碰撞"这种场景比手写 broadcast 更省代码，且 Claude 对 Colyseus 模板极度熟悉。

**明确不推荐**：Nakama（对单人开发过重）、Photon（Web 弱、AI 差）、Liveblocks 做游戏主循环（按分钟计费在长在线场景会爆），Socket.io 从零手撸（原型阶段每个特性都要重写）。

Sources:
- [Colyseus Pricing](https://colyseus.io/pricing/) · [Colyseus Docs](https://docs.colyseus.io/) · [Colyseus Cloud Pricing](https://docs.colyseus.io/cloud/pricing-billing)
- [PartyKit](https://www.partykit.io/) · [Deploy PartyKit to Cloudflare](https://docs.partykit.io/guides/deploy-to-cloudflare/) · [PartyKit joins Cloudflare](https://blog.partykit.io/posts/partykit-is-joining-cloudflare/)
- [Cloudflare Durable Objects Pricing](https://developers.cloudflare.com/durable-objects/platform/pricing/)
- [Liveblocks Pricing](https://liveblocks.io/pricing) · [Liveblocks Free Plan](https://liveblocks.io/docs/pricing/plans/free.md) · [Fairer Billing First Day Free](https://liveblocks.io/blog/introducing-fairer-billing-with-first-day-free)
- [Supabase Realtime Multiplayer](https://supabase.com/blog/supabase-realtime-with-multiplayer-features) · [Supabase Realtime Docs](https://supabase.com/docs/guides/realtime) · [Supabase Broadcast](https://supabase-supabase.mintlify.app/realtime/broadcast) · [Supabase Presence](https://supabase.com/docs/guides/realtime/presence)
- [Heroic Cloud (Nakama) Pricing](https://heroiclabs.com/pricing/) · [Nakama GitHub](https://github.com/heroiclabs/nakama) · [Nakama vs PlayFab](https://codewizards.io/nakama-vs-playfab-online-player-services/)