//! Binary asset storage (切片区地基：存 key 不存 URL)。
//!
//! `AssetStore` trait：`put` / `get` / `delete` + `public_url`。
//! `LocalAssetStore`：根目录 `data/assets/`，字节落盘。
//! `public_url(key)` 返回 `/api/assets/{key}` —— 唯一允许拼 URL 的地方。
//!
//! 路径穿越防护：任何含 `..` 或绝对路径分量（盘符 `C:` 等）的 key 一律拒绝。
//! 不引入 `async-trait`：用手写 `Pin<Box< dyn Future + Send >>`（等价展开），
//! 保持 `Arc<dyn AssetStore + Send + Sync>` 可作 trait object。

use std::future::Future;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;

/// 二进制资产存储抽象。可 send + sync，以 `Arc<dyn AssetStore + Send + Sync>` 放 `AppState`。
/// 不引入 `async-trait`：手写 `Pin<Box< dyn Future + Send >>`（等价展开）。
#[allow(clippy::type_complexity)] // 手写 boxed future 是 async_trait 的等价展开
pub trait AssetStore: Send + Sync {
    /// 存入字节。
    fn put<'a>(
        &'a self,
        key: &'a str,
        bytes: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<(), AssetError>> + Send + 'a>>;

    /// 取出字节（None = 不存在）。
    fn get<'a>(
        &'a self,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Vec<u8>>, AssetError>> + Send + 'a>>;

    /// 删除（不存在视为成功）。
    fn delete<'a>(
        &'a self,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), AssetError>> + Send + 'a>>;

    /// 返回前端可访问的 URL（`/api/assets/{key}`）。
    fn public_url(&self, key: &str) -> String {
        format!("/api/assets/{key}")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("invalid key: path traversal or absolute component")]
    InvalidKey,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// 本地文件系统实现。根目录默认 `data/assets/`。
pub struct LocalAssetStore {
    root: PathBuf,
}

impl LocalAssetStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// 校验 key 无穿越/绝对分量，返回绝对路径。
    fn resolve(&self, key: &str) -> Result<PathBuf, AssetError> {
        let p = Path::new(key);
        for c in p.components() {
            match c {
                Component::ParentDir => return Err(AssetError::InvalidKey),
                Component::RootDir => return Err(AssetError::InvalidKey),
                Component::Prefix(_) => return Err(AssetError::InvalidKey), // 盘符 C:
                _ => {}
            }
        }
        // 额外拒绝 `..` 字面量（理论上 components 已覆盖，但防御）
        if key.contains("..") {
            return Err(AssetError::InvalidKey);
        }
        let full = self.root.join(p);
        Ok(full)
    }
}

impl AssetStore for LocalAssetStore {
    fn put<'a>(
        &'a self,
        key: &'a str,
        bytes: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<(), AssetError>> + Send + 'a>> {
        Box::pin(async move {
            let path = self.resolve(key)?;
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(&path, bytes).await?;
            Ok(())
        })
    }

    fn get<'a>(
        &'a self,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Vec<u8>>, AssetError>> + Send + 'a>> {
        Box::pin(async move {
            let path = self.resolve(key)?;
            match tokio::fs::read(&path).await {
                Ok(b) => Ok(Some(b)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(e.into()),
            }
        })
    }

    fn delete<'a>(
        &'a self,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), AssetError>> + Send + 'a>> {
        Box::pin(async move {
            let path = self.resolve(key)?;
            match tokio::fs::remove_file(&path).await {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e.into()),
            }
        })
    }
}

/// 按扩展名映射 content-type（不加新依赖）。
pub fn content_type_for_ext(key: &str) -> &'static str {
    let ext = Path::new(key)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "json" => "application/json",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
}

/// 给 key 生成随机 ID（十六进制 16 字节）。
pub fn random_asset_id() -> String {
    use rand_core::{OsRng, RngCore};
    let mut buf = [0u8; 16];
    OsRng.fill_bytes(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_traversal() {
        let store = LocalAssetStore::new("/tmp/hoi-test-assets");
        assert!(store.resolve("avatar/a.png").is_ok());
        assert!(store.resolve("../etc/passwd").is_err());
        assert!(store.resolve("a/../../b.png").is_err());
        assert!(store.resolve("/etc/passwd").is_err());
        // Windows-style absolute path (C:) — Prefix component only on Windows,
        // but `..` literal check on Unix + general hardening is sufficient.
        #[cfg(windows)]
        assert!(store.resolve("C:/evil.png").is_err());
    }

    #[tokio::test]
    async fn put_get_delete_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let store = LocalAssetStore::new(tmp.path());
        let key = "avatar/u1/south.png";
        store.put(key, b"hello").await.unwrap();
        assert_eq!(store.get(key).await.unwrap(), Some(b"hello".to_vec()));
        store.delete(key).await.unwrap();
        assert_eq!(store.get(key).await.unwrap(), None);
    }

    #[test]
    fn content_type_mapping() {
        assert_eq!(content_type_for_ext("a/b.png"), "image/png");
        assert_eq!(content_type_for_ext("a.JPEG"), "image/jpeg");
        assert_eq!(content_type_for_ext("noext"), "application/octet-stream");
    }

    #[test]
    fn public_url_format() {
        let store = LocalAssetStore::new("/tmp");
        assert_eq!(store.public_url("avatar/u1/south.png"), "/api/assets/avatar/u1/south.png");
    }
}
