// SPDX-License-Identifier: AGPL-3.0-only
//! Nginx 主配置可视化：结构模型与历史版本的文件存储
//!
//! 这是 [`crate::storage::conf`] 统一文件存储接口的第一个消费方，目录为
//! `data/conf/nginx/`：
//!
//! ```text
//! {ZAP_PATH}/data/conf/nginx/
//! ├── nginx_conf.yaml          # 当前结构模型（YAML）
//! └── revisions/
//!     ├── index.yaml           # 历史版本元数据列表
//!     └── {ts}.conf            # 每次 apply 渲染出的 nginx.conf 全文
//! ```

use crate::storage::conf::{ProjectConf, RevisionMeta};
use crate::zap::ZapError;
use serde_json::Value;

/// 模型文件名（YAML）。
pub const SPEC_FILE: &str = "nginx_conf.yaml";

/// 取本项目的文件仓库（目录 `data/conf/nginx`）。
fn store() -> ProjectConf {
    ProjectConf::new("nginx")
}

/// 写入结构模型（草稿 / 接管回填共用）。
pub async fn write_spec(spec: &Value) -> Result<(), ZapError> {
    store().write_yaml(SPEC_FILE, spec).await
}

/// 读取结构模型；文件不存在返回 `Ok(None)`，由调用方决定是否向 zapexec 接管现有配置。
pub async fn read_spec() -> Result<Option<Value>, ZapError> {
    store().read_yaml(SPEC_FILE).await
}

/// 留存一次 apply 渲染出的 nginx.conf 全文，返回版本 id。
pub async fn save_revision(content: &str, actor: &str, ok: bool) -> Result<i64, ZapError> {
    store().save_revision(content, actor, ok).await
}

/// 最近 30 条历史版本元数据（按时间倒序）。
pub async fn list_revisions() -> Result<Vec<RevisionMeta>, ZapError> {
    store().list_revisions().await
}

/// 读取某一历史版本的 nginx.conf 全文。
pub async fn get_revision(id: i64) -> Result<Option<String>, ZapError> {
    store().get_revision(id).await
}
