# Rust Web 框架选型对比（2025-2026）

## 1. Axum 0.8.x（Tokio 官方）
- crate: `axum = "0.8"` — https://docs.rs/axum/0.8
- 配套: `tower = "0.5"`, `tower-http = "0.6"`, `tokio = "1.42"`
- 依赖数：中等（含 tokio 约 80-100 个），release 二进制 ~5-8 MB（strip 后 3-5 MB）
- WebSocket：原生 `axum::extract::ws`，无需额外 crate
- Extractor：`State<T>`, `Path<T>`, `Json<T>`, `Query<T>`，类型化路由 `Router::new().route("/x", get(handler))`
- Middleware：完整 tower/tower-http 生态（CORS、trace、compression、timeout、限流）
- AI 友好度：★★★★★ 最高，Claude 训练语料最多，官方 examples 仓库 40+ 个可运行示例
- 一体化模板：无官方 starter，但社区模板极多（axum-login、shuttle-axum、rust-axum-clean）
- 生态：Tokio 官方背书，2025 已成事实标准

## 2. Actix-web 4.9.x
- crate: `actix-web = "4.9"` — https://docs.rs/actix-web/4.9
- 依赖数：稍多（自带 actix runtime），二进制 ~6-9 MB
- WebSocket：`actix-ws = "0.3"` 或 `actix-web-actors`（略绕）
- Extractor：`web::Data<T>`, `web::Path`, `web::Json`，宏路由 `#[get("/x")]`
- Middleware：自有中间件系统（不兼容 tower），生态自成一派
- AI 友好度：★★★★☆ 语料多，但 3.x/4.x 混杂，Claude 偶尔会写出旧 API
- 一体化模板：无 first-party starter
- 特色：Techempower benchmark 常年前三，但对单人小项目性能是过剩的

## 3. Rocket 0.5.1
- crate: `rocket = "0.5.1"` — https://docs.rs/rocket/0.5.1
- 依赖数：中等，二进制 ~5-7 MB
- WebSocket：**无原生支持**，需 `rocket_ws`（社区，不太活跃）
- Extractor：宏路由 `#[get("/x/<id>")]`，语法糖最甜
- AI 友好度：★★★☆☆ 老版本同步语法污染语料，Claude 常写出 0.4 sync 代码
- 生态：async 转型晚（0.5 才 async），社区热度下降
- 结论：新项目不推荐，尤其需要 WS

## 4. Poem 3.x
- crate: `poem = "3.1"` — https://docs.rs/poem/3.1
- OpenAPI: `poem-openapi = "5.1"`
- 二进制 ~5-7 MB
- WebSocket：原生支持
- 亮点：`poem-openapi` 是 Rust 生态中最好的类型化 OpenAPI 生成器（比 utoipa 更纯）
- AI 友好度：★★★☆☆ 语料量中等，Claude 会写但偶有幻觉
- 中文社区活跃（作者 sunli）

## 5. Warp 0.3.7
- crate: `warp = "0.3.7"` — https://docs.rs/warp/0.3.7
- filter 组合式路由，函数式但类型错误信息极其难读
- WebSocket：原生
- AI 友好度：★★☆☆☆ 类型体操让 Claude 经常卡在 trait bound
- 现状：维护缓慢，社区基本迁移到 Axum

## 6. Loco.rs 0.14.x
- crate: `loco-rs = "0.14"` — https://docs.rs/loco-rs/0.14
- 基于 Axum + SeaORM，Rails-like，内置 auth/mailer/scheduler/BG worker/CLI 脚手架
- 二进制 ~10-15 MB（东西多）
- WebSocket：底层 Axum 原生
- AI 友好度：★★★☆☆ 语料在快速积累，`loco new` 脚手架规范，Claude 顺着写就行
- **对一人开发极友好**：`cargo loco generate scaffold`、内置用户/session/JWT
- 风险：还年轻（<1.0），breaking change 偶发

## 7. Salvo 0.76.x
- crate: `salvo = "0.76"` — https://docs.rs/salvo/0.76
- 中文文档最全（salvo.rs），中国团队维护
- 二进制 ~5-7 MB，WebSocket/OpenAPI/ACME 都内置
- AI 友好度：★★★☆☆ 英文语料少于 Axum，中文语料丰富
- 生态：北美圈子小众

## 8. Pavex（新生代）
- 仍在 alpha（2026 中），编译期依赖注入 + AOT 代码生成
- 二进制超小、性能顶级，但 API 频繁变动
- AI 友好度：★☆☆☆☆ 训练数据几乎为零，Claude 会瞎编
- 结论：生产不用

## 9. 极简 hyper / tiny_http
- `hyper = "1.5"` 裸写：需要自己拼 router/state，二进制 ~2-3 MB
- `tiny_http = "0.12"`：sync 阻塞式，不适合 WebSocket + 多人游戏
- AI 友好度：hyper 语料多但拼装步骤长，Claude 容易写出泄漏 handle

---

## 首选：**Axum 0.8**

一句话理由：Tokio 官方 + tower 中间件生态 + 原生 WebSocket + Claude 训练语料最丰富，一人 vibe coding 最不容易踩坑。

### 最小可跑骨架 `src/main.rs`

```rust
use axum::{
    extract::{ws::WebSocketUpgrade, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::{services::ServeDir, trace::TraceLayer};

#[derive(Clone)]
struct AppState { /* db pool, broadcast tx, ... */ }

#[derive(Deserialize)] struct LoginReq { name: String }
#[derive(Serialize)]   struct LoginRes { token: String }

async fn login(Json(req): Json<LoginReq>) -> Json<LoginRes> {
    Json(LoginRes { token: format!("tok-{}", req.name) })
}

async fn ws(ws: WebSocketUpgrade, State(_s): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        while let Some(Ok(msg)) = socket.recv().await {
            let _ = socket.send(msg).await; // echo
        }
    })
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {});
    let app = Router::new()
        .route("/api/login", post(login))
        .route("/ws", get(ws))
        .fallback_service(ServeDir::new("dist"))
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

`Cargo.toml` 关键依赖：
```
axum = { version = "0.8", features = ["ws", "macros"] }
tokio = { version = "1.42", features = ["full"] }
tower-http = { version = "0.6", features = ["fs", "trace", "cors"] }
serde = { version = "1", features = ["derive"] }
```

若后期想要 Rails 式脚手架省时间，可以整体迁到 **Loco.rs**（底层还是 Axum，代码基本兼容）作为 B 选项保留。