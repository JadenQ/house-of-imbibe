# 用户资产存储方案调研

## 一、本地文件系统 + tower-http ServeDir

### tower-http `v0.6.2`
- docs.rs: https://docs.rs/tower-http/0.6.2/tower_http/services/struct.ServeDir.html
- 功能覆盖：静态目录服务、Range 请求、ETag/Last-Modified、gzip/br 预压缩文件自动选择、fallback 到 index.html
- 二进制大小/内存：几乎零成本，仅一个 tower Service，本身几十 KB
- 上手难度：极低，`Router::new().nest_service("/assets", ServeDir::new("./data/assets"))` 一行
- AI 友好度：非常高，axum + tower-http 文档在 Claude 训练语料里覆盖度顶级
- 生态活跃度：hyperium 官方维护，每月更新

### 配合 mime_guess `v2.0.5`
- docs.rs: https://docs.rs/mime_guess/2.0.5/mime_guess/
- 上传时按扩展名判断 Content-Type

**问题**：单机 VPS 挂盘，备份/CDN/多机扩展都要自己搞；PNG 命中率高但没 CDN 就是回源，跨洲用户慢。

---

## 二、object_store crate（推荐核心抽象层）

### object_store `v0.11.2`
- docs.rs: https://docs.rs/object_store/0.11.2/object_store/
- 功能覆盖：统一 `ObjectStore` trait，`LocalFileSystem` / `AmazonS3` / `GoogleCloudStorage` / `MicrosoftAzure` / `InMemory` / `Http`（WebDAV/R2 也走 S3 兼容）
- 二进制大小：默认约 1-2 MB（含 reqwest+rustls），可用 features 精简到只留 aws
- 上手难度：低，`store.put(&path, bytes).await` / `store.get(&path).await` 全后端一致
- AI 友好度：高，Apache Arrow / DataFusion 生态在用，示例多
- 生态活跃度：Apache 顶级项目维护，月发版

**关键卖点**：这就是"MVP 本地磁盘 → 上线 R2 不改业务代码"的最佳答案。

---

## 三、Cloudflare R2 直连

R2 是 S3 兼容 API，**没有出口流量费**，适合像素资产（大量小 PNG 频繁读）。

### 走 object_store 的 AmazonS3 后端
```rust
AmazonS3Builder::new()
    .with_endpoint("https://<accountid>.r2.cloudflarestorage.com")
    .with_bucket_name("assets")
    .with_region("auto")
    .with_access_key_id(...).with_secret_access_key(...)
    .build()
```
- 走 rustls 即可，不用 aws-sdk-s3 那套重家伙
- R2 还能挂 custom domain + Cloudflare CDN，直接免费加速全球

### 备选：aws-sdk-s3 `v1.68`
- docs.rs: https://docs.rs/aws-sdk-s3/1.68.0/aws_sdk_s3/
- 功能最全（multipart / presigned / lifecycle 全支持）
- 二进制体积重（+5-8 MB），依赖树深（`aws-config`/`aws-smithy-*` 一大坨）
- AI 友好度：训练语料多，但对小项目过度
- **不推荐**：object_store 已经够用，除非要用 S3 高级特性

---

## 四、S3 兼容自托管

### MinIO
- 自托管 S3 兼容服务，Go 写的单二进制
- 适合：想完全掌握数据、有内网机器
- 代价：多一台服务/一个 docker，还是要备份

### Backblaze B2
- S3 兼容 API，价格便宜（$0.006/GB 存储，$0.01/GB 出口，前 3× 存储量出口免费）
- 无 CDN，但可挂 Cloudflare 免费带宽联盟走 R2/CF 免出口

**建议**：这两个都用 object_store 的 AmazonS3 后端接，MVP 阶段没必要，规模上来再说。

---

## 五、前端直传 vs Rust 代理

### 前端直传（presigned URL）
- 用 object_store 的 [`signer::Signer`](https://docs.rs/object_store/0.11.2/object_store/signer/trait.Signer.html) trait，AWS/R2 后端实现了它
- 后端只发一个 15 分钟有效期的 PUT URL，浏览器直传 R2
- **优点**：Rust 进程不过流量，内存/CPU 零负担；带宽账单由 R2 承担（还免费）
- **缺点**：无法在流中做图像处理/病毒扫描，得靠后置 webhook 或客户端预处理

### Rust 代理上传
- multer `v3.1.0`（docs.rs: https://docs.rs/multer/3.1.0/multer/）解析 multipart
- 边流边处理（压缩、像素化、限制尺寸），处理完再 put 到 store
- **优点**：安全可控，可以直接生成 sprite sheet 后存
- **缺点**：Rust 进程要吃掉整份上传流量

**推荐组合**：
- 头像/照片上传（要像素化处理）→ **走 Rust 代理**（图片小，几百 KB 无压力）
- fal.ai 生图结果 → **后端拉回本地存**（下节详述）
- 未来大文件（自定义 sprite 上传大包）→ presigned 直传

---

## 六、图像处理

### image `v0.25.5`
- docs.rs: https://docs.rs/image/0.25.5/image/
- 覆盖：PNG/JPEG/WebP 编解码、resize、crop、格式转换
- 上手最简单，Claude 写起来行云流水

### fast_image_resize `v5.1.0`
- docs.rs: https://docs.rs/fast_image_resize/5.1.0/fast_image_resize/
- SIMD 加速的 resize，比 image::imageops::resize 快 4-15 倍
- 适合：批量像素化时性能敏感场景
- 上手难度：中，API 比 image 复杂一点，但 AI 能写

### rgb `v0.8.50`
- docs.rs: https://docs.rs/rgb/0.8.50/rgb/
- 只是 `RGB<u8>` / `RGBA<u8>` 类型定义，配合上面两个用
- 像素化算法（k-means 调色板、量化）时用得上

### 补充：imagequant `v4.3`
- docs.rs: https://docs.rs/imagequant/4.3.4/imagequant/
- pngquant 同款算法，做像素风调色板量化极佳（GBA 风格必备）
- oxipng `v9.1` 后处理压缩 PNG

**像素化流水线建议**：
```
上传 -> image::load -> fast_image_resize 缩到 64×64 -> 
imagequant 量化到 32 色 -> image::save PNG -> object_store::put
```

---

## 七、fal.ai 生图结果对接

fal.ai 返回的是**临时 CDN URL**（一般 24h-7d 过期），必须拉回来存。

### 拉回方案
- reqwest `v0.12.9` (rustls-tls, stream feature) 流式下载
- 直接 `store.put_multipart()` 边下边写，不占内存
- 存到 `assets/generated/{userid}/{uuid}.png`

### 拉回时机
- **同步**：用户点"确认使用" → 后端拉 → 返回自家 URL；简单但有 3-5 s 等待
- **异步**：立即返回 fal URL 给前端，后台 task 拉回并更新数据库；用户体验好

**推荐同步**，一人开发别搞消息队列，用户等 3 秒可接受。

---

## 八、MVP → 上线平滑过渡设计

**核心：所有业务代码只依赖 `Arc<dyn ObjectStore>`。**

### 抽象层
```rust
// AppState
pub struct AppState {
    pub assets: Arc<dyn ObjectStore>,
    pub public_base_url: String,  // MVP: "http://localhost:3000/assets"
                                  // 上线: "https://cdn.yourdomain.com"
}
```

### MVP 阶段（本地磁盘）
```rust
let store = LocalFileSystem::new_with_prefix("./data/assets")?;
// 另外挂 ServeDir 提供 HTTP 访问
app.nest_service("/assets", ServeDir::new("./data/assets"))
```
- 单 VPS 一切齐活，数据在 `./data/assets`
- 备份 = `rsync ./data`

### 上线阶段（R2）
```rust
let store = AmazonS3Builder::from_env()
    .with_endpoint(env::var("R2_ENDPOINT")?)
    .with_bucket_name("hoi-assets")
    .build()?;
// 前端直接访问 https://cdn.yourdomain.com/<path>（R2 自定义域名）
```
- 业务代码一行不改
- 迁移工具：写个 30 行 CLI，遍历本地 store 的 `list()`，逐个 `put` 到 R2

### URL 生成函数
```rust
// 业务代码只调这一个函数拿 URL，绝不硬编码
fn asset_url(state: &AppState, path: &Path) -> String {
    format!("{}/{}", state.public_base_url, path)
}
```

### 环境变量切换
```
# .env.dev
ASSET_BACKEND=local
ASSET_LOCAL_DIR=./data/assets
PUBLIC_BASE_URL=http://localhost:3000/assets

# .env.prod
ASSET_BACKEND=r2
R2_ENDPOINT=https://xxx.r2.cloudflarestorage.com
R2_BUCKET=hoi-assets
PUBLIC_BASE_URL=https://cdn.yourdomain.com
```

`main.rs` 里一个 match 分支构造出 `Arc<dyn ObjectStore>` 塞进 AppState 即可。

---

## 首选方案

**`object_store` (0.11) + LocalFileSystem 起步 + 无缝切 R2**

**图像处理：`image` (0.25) + `fast_image_resize` (5.1) + `imagequant` (4.3)**

**上传路径：Rust 代理 multipart（头像/照片）+ 后端同步拉回 fal.ai 结果**

理由：
1. **零迁移成本**：object_store 的 trait 抽象天生为"本地→云"设计，Apache 生态背书，靠谱
2. **依赖极轻**：MVP 全程零外部服务，一个 VPS + `./data` 目录就能跑，二进制里也就多个 1-2 MB
3. **上线路径明确**：R2 免出口 + 自定义域名 + Cloudflare CDN，全球加速零账单焦虑，比 S3/B2 都优
4. **AI 好写**：object_store 和 image crate 都是 Claude 训练语料里的高频 crate，你让它写 handler 基本一遍过
5. **像素游戏特化**：`imagequant` 做调色板量化是 GBA 风格的关键武器，绿宝石全屏也就 240 色以内
6. **拒绝过度设计**：不上 MinIO（多一台服务）、不上 aws-sdk-s3（依赖太重）、不上消息队列拉 fal.ai（用户等 3 秒可接受），一人 vibe coding 就该这样