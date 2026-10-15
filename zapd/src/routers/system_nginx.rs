// SPDX-License-Identifier: AGPL-3.0-only
//! Nginx 服务配置与运行状态（服务器配置 → Nginx 配置 / 服务器状态 → Nginx Server）。
//!
//! 端点（均需管理员）：
//! - GET  /system/nginx/status            状态探测（未安装时 installed=false）
//! - GET  /system/nginx/config            列出可编辑配置（主配置 + conf 白名单）
//! - GET  /system/nginx/config/content    读取指定配置内容（Query: path）
//! - POST /system/nginx/config/save       保存配置（备份 → 写入 → nginx -t → 回滚/重载）
//! - POST /system/nginx/control           服务控制 start/stop/restart/reload
//! - POST /system/nginx/default-vhost     设置默认站点（IP 访问开关）{ enable }
//! - GET  /system/nginx/stub-status       查询状态页 stub_status 并采集指标
//! - POST /system/nginx/stub-status       开启 / 关闭状态页 { enable }

use std::net::SocketAddr;

use axum::Json;
use axum::extract::{Extension, Query};
use serde::Deserialize;
use serde_json::json;

use crate::storage::nginx_conf as nginx_conf_storage;
use crate::zap::ZapError;
use crate::zap::ZapJsonResult;
use crate::zap::audit;
use crate::zap::jwt::ValidatedClaims;
use crate::zap::jwt::is_admin;
use zap_proto::{NGINX_LOG_FORMAT_JSON, NginxGlobalSpec, Request};

fn require_admin(claims: &ValidatedClaims) -> Result<(), ZapError> {
    if is_admin(claims) {
        Ok(())
    } else {
        Err(ZapError::New(-1, "仅管理员可访问".to_string()))
    }
}

/// 执行一次 zapexec 请求，透传其 code/message/data。
async fn exec(req: Request) -> Result<Json<serde_json::Value>, ZapError> {
    let resp = crate::zapexec::call(req).await?;
    if resp.code != 0 {
        return Err(ZapError::New(resp.code, resp.message));
    }
    Ok(Json(
        json!({ "code": 0, "message": "ok", "data": resp.data }),
    ))
}

/// GET /system/nginx/status
pub async fn nginx_status(claims: ValidatedClaims) -> ZapJsonResult {
    require_admin(&claims)?;
    exec(Request::NginxStatus).await
}

/// GET /system/nginx/config
pub async fn nginx_conf_list(claims: ValidatedClaims) -> ZapJsonResult {
    require_admin(&claims)?;
    exec(Request::NginxConfList).await
}

#[derive(Debug, Deserialize)]
pub struct NginxReadQuery {
    pub path: String,
}

/// GET /system/nginx/config/content?path=...
pub async fn nginx_conf_read(
    claims: ValidatedClaims,
    Query(q): Query<NginxReadQuery>,
) -> ZapJsonResult {
    require_admin(&claims)?;
    exec(Request::NginxConfRead { path: q.path }).await
}

#[derive(Debug, Deserialize)]
pub struct NginxSaveBody {
    pub path: String,
    pub content: String,
}

/// POST /system/nginx/config/save
pub async fn nginx_conf_save(
    claims: ValidatedClaims,
    client_addr: Extension<SocketAddr>,
    Json(body): Json<NginxSaveBody>,
) -> ZapJsonResult {
    require_admin(&claims)?;
    let result = exec(Request::NginxConfSave {
        path: body.path.clone(),
        content: body.content.clone(),
    })
    .await;
    if result.is_ok() {
        audit::log(
            Some(&claims),
            Some(client_addr.ip().to_string().as_str()),
            "nginx_conf_save",
            "nginx",
            &format!("保存 Nginx 配置 {}", body.path),
        )
        .await;
    }
    result
}

#[derive(Debug, Deserialize)]
pub struct NginxControlBody {
    pub action: String,
}

/// POST /system/nginx/control
pub async fn nginx_control(
    claims: ValidatedClaims,
    client_addr: Extension<SocketAddr>,
    Json(body): Json<NginxControlBody>,
) -> ZapJsonResult {
    require_admin(&claims)?;
    if !matches!(
        body.action.as_str(),
        "start" | "stop" | "restart" | "reload"
    ) {
        return Err(ZapError::New(
            -1,
            "仅支持 start / stop / restart / reload".to_string(),
        ));
    }
    let result = exec(Request::NginxControl {
        action: body.action.clone(),
    })
    .await;
    if result.is_ok() {
        audit::log(
            Some(&claims),
            Some(client_addr.ip().to_string().as_str()),
            "nginx_control",
            "nginx",
            &format!("Nginx 服务操作 {}", body.action),
        )
        .await;
    }
    result
}

/// GET /system/nginx/stub-status（查询状态页并采集指标）
pub async fn nginx_stub_status_get(claims: ValidatedClaims) -> ZapJsonResult {
    require_admin(&claims)?;
    exec(Request::NginxStubStatus { enable: None }).await
}

#[derive(Debug, Deserialize)]
pub struct NginxStubStatusBody {
    pub enable: bool,
}

/// POST /system/nginx/stub-status（开启 / 关闭状态页）
pub async fn nginx_stub_status_set(
    claims: ValidatedClaims,
    client_addr: Extension<SocketAddr>,
    Json(body): Json<NginxStubStatusBody>,
) -> ZapJsonResult {
    require_admin(&claims)?;
    let result = exec(Request::NginxStubStatus {
        enable: Some(body.enable),
    })
    .await;
    if result.is_ok() {
        audit::log(
            Some(&claims),
            Some(client_addr.ip().to_string().as_str()),
            "nginx_stub_status",
            "nginx",
            if body.enable {
                "启用 Nginx 状态页 stub_status"
            } else {
                "关闭 Nginx 状态页 stub_status"
            },
        )
        .await;
    }
    result
}

#[derive(Debug, Deserialize)]
pub struct NginxDefaultVhostBody {
    pub enable: bool,
}

/// POST /system/nginx/default-vhost
pub async fn nginx_default_vhost(
    claims: ValidatedClaims,
    client_addr: Extension<SocketAddr>,
    Json(body): Json<NginxDefaultVhostBody>,
) -> ZapJsonResult {
    require_admin(&claims)?;
    let result = exec(Request::NginxDefaultVhost {
        enable: body.enable,
    })
    .await;
    if result.is_ok() {
        audit::log(
            Some(&claims),
            Some(client_addr.ip().to_string().as_str()),
            "nginx_default_vhost",
            "nginx",
            if body.enable {
                "开启默认站点（IP / 未匹配域名显示欢迎页）"
            } else {
                "关闭默认站点（IP / 未匹配域名直接断开）"
            },
        )
        .await;
    }
    result
}

// ── 主配置可视化（结构模型 → 全量渲染 nginx.conf）────────────────
//
// 模型不再进数据库，落在 `{ZAP_PATH}/data/conf/nginx/nginx_conf.yaml`；
// 历史版本在 `data/conf/nginx/revisions/`。nginx.conf 本身是渲染产物、在磁盘。
// 首次读取无记录时向执行端索取：`nginx.global_spec` 会解析现有 nginx.conf
// 回填模型（识别不了的行进 preserve_*_raw），取不到就用面板默认模型。

use axum::extract::Path;
use serde_json::Value;

/// 执行一次 zapexec 请求并返回 data（失败保留原始 message）。
async fn exec_data(req: Request) -> Result<Value, ZapError> {
    let resp = crate::zapexec::call(req).await?;
    if resp.code != 0 {
        return Err(ZapError::New(resp.code, resp.message));
    }
    Ok(resp.data.unwrap_or(Value::Null))
}

/// 持久化模型（草稿 / 接管回填共用），改为 `{ZAP_PATH}/data/nginx/nginx_conf.yaml`。
async fn save_spec(spec: &Value) -> Result<(), ZapError> {
    nginx_conf_storage::write_spec(spec).await
}

/// 读取模型；无文件时按现有 nginx.conf（或默认值）生成一份写入文件。
async fn load_or_seed_spec() -> Result<(Value, bool), ZapError> {
    if let Some(spec) = nginx_conf_storage::read_spec().await? {
        // 已有草稿文件若缺少 JSON log_format，补入默认值并持久化
        let mut spec_obj: NginxGlobalSpec =
            serde_json::from_value(spec.clone()).unwrap_or_default();
        if spec_obj
            .http
            .logging
            .log_format_json
            .as_deref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            spec_obj.http.logging.log_format_json = Some(NGINX_LOG_FORMAT_JSON.to_string());
            let normalized = serde_json::to_value(&spec_obj)
                .map_err(|e| ZapError::New(-1, format!("模型序列化失败: {e}")))?;
            save_spec(&normalized).await?;
            return Ok((normalized, false));
        }
        return Ok((spec, false));
    }
    let d = exec_data(Request::NginxGlobalSpec).await?;
    let spec = d.get("spec").cloned().unwrap_or_else(|| json!({}));
    save_spec(&spec).await?;
    Ok((spec, true))
}

/// GET /system/nginx/global/spec
pub async fn nginx_global_spec_get(claims: ValidatedClaims) -> ZapJsonResult {
    require_admin(&claims)?;
    let (spec, seeded) = load_or_seed_spec().await?;
    Ok(Json(
        json!({ "code": 0, "message": "ok", "data": { "spec": spec, "seeded": seeded } }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct NginxGlobalSpecBody {
    pub spec: Value,
}

/// PUT /system/nginx/global/spec —— 只存草稿，不动 nginx.conf
pub async fn nginx_global_spec_put(
    claims: ValidatedClaims,
    client_addr: Extension<SocketAddr>,
    Json(body): Json<NginxGlobalSpecBody>,
) -> ZapJsonResult {
    require_admin(&claims)?;
    if !body.spec.is_object() {
        return Err(ZapError::New(-1, "配置模型必须是 JSON 对象".to_string()));
    }
    save_spec(&body.spec).await?;
    audit::log(
        Some(&claims),
        Some(client_addr.ip().to_string().as_str()),
        "nginx_global_spec_save",
        "nginx",
        "保存 Nginx 主配置模型（未落盘）",
    )
    .await;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": null })))
}
/// POST /system/nginx/global/preview —— 干跑：渲染全文 + 冲突告警 + diff
pub async fn nginx_global_preview(
    claims: ValidatedClaims,
    Json(body): Json<NginxGlobalSpecBody>,
) -> ZapJsonResult {
    require_admin(&claims)?;
    let spec_json = serde_json::to_string(&body.spec).unwrap_or_else(|_| "{}".to_string());
    let d = exec_data(Request::NginxGlobalPreview { spec_json }).await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": d })))
}

/// POST /system/nginx/global/apply —— 渲染 → 校验 → 备份替换 → reload，并留 revision
pub async fn nginx_global_apply(
    claims: ValidatedClaims,
    client_addr: Extension<SocketAddr>,
    Json(body): Json<NginxGlobalSpecBody>,
) -> ZapJsonResult {
    require_admin(&claims)?;
    let spec_json = serde_json::to_string(&body.spec).unwrap_or_else(|_| "{}".to_string());
    let result = exec_data(Request::NginxGlobalApply { spec_json }).await;
    if let Ok(d) = &result {
        let content = d.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let _ = nginx_conf_storage::save_revision(content, &claims.sub, true).await;
        save_spec(&body.spec).await?;
        audit::log(
            Some(&claims),
            Some(client_addr.ip().to_string().as_str()),
            "nginx_global_apply",
            "nginx",
            "应用 Nginx 主配置并重载",
        )
        .await;
    }
    let d = result?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": d })))
}

/// POST /system/nginx/global/adopt —— 按现有 nginx.conf 重新回填模型
pub async fn nginx_global_adopt(
    claims: ValidatedClaims,
    client_addr: Extension<SocketAddr>,
) -> ZapJsonResult {
    require_admin(&claims)?;
    let d = exec_data(Request::NginxGlobalSpec).await?;
    let spec = d.get("spec").cloned().unwrap_or_else(|| json!({}));
    save_spec(&spec).await?;
    audit::log(
        Some(&claims),
        Some(client_addr.ip().to_string().as_str()),
        "nginx_global_adopt",
        "nginx",
        "按现有 nginx.conf 重新接管主配置模型",
    )
    .await;
    Ok(Json(
        json!({ "code": 0, "message": "ok", "data": { "spec": spec } }),
    ))
}

/// GET /system/nginx/global/revisions
pub async fn nginx_global_revisions(claims: ValidatedClaims) -> ZapJsonResult {
    require_admin(&claims)?;
    let rows = nginx_conf_storage::list_revisions().await?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|m| json!({ "id": m.id, "actor": m.actor, "ok": m.ok, "created_at": m.created_at }))
        .collect();
    Ok(Json(
        json!({ "code": 0, "message": "ok", "data": { "items": items } }),
    ))
}

/// GET /system/nginx/global/revisions/:id —— 取回某一版全文
pub async fn nginx_global_revision_get(
    claims: ValidatedClaims,
    Path(id): Path<i64>,
) -> ZapJsonResult {
    require_admin(&claims)?;
    let content = nginx_conf_storage::get_revision(id)
        .await?
        .ok_or_else(|| ZapError::New(-1, "该历史版本不存在".to_string()))?;
    Ok(Json(
        json!({ "code": 0, "message": "ok", "data": { "content": content } }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct NginxRevisionRollbackBody {
    pub id: i64,
}

/// POST /system/nginx/global/rollback —— 还原历史版本。
///
/// 复用既有 `nginx.conf_save` 链路：备份 → 写入 → `nginx -t` → 失败回滚 → reload。
pub async fn nginx_global_rollback(
    claims: ValidatedClaims,
    client_addr: Extension<SocketAddr>,
    Json(body): Json<NginxRevisionRollbackBody>,
) -> ZapJsonResult {
    require_admin(&claims)?;
    let content = nginx_conf_storage::get_revision(body.id)
        .await?
        .ok_or_else(|| ZapError::New(-1, "该历史版本不存在".to_string()))?;
    let status = exec_data(Request::NginxStatus).await?;
    let Some(conf) = status.get("conf_file").and_then(|v| v.as_str()) else {
        return Err(ZapError::New(-1, "未探测到 Nginx 主配置路径".to_string()));
    };
    // 结果丢弃：写入成功与否由 ConfSave 内部跑 `nginx -t` 失败回滚保证
    let _ = exec(Request::NginxConfSave {
        path: conf.to_string(),
        content,
    })
    .await?;
    audit::log(
        Some(&claims),
        Some(client_addr.ip().to_string().as_str()),
        "nginx_global_rollback",
        "nginx",
        &format!("回滚 Nginx 主配置到历史版本 id={}", body.id),
    )
    .await;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": null })))
}

/// GET /system/nginx/global/zones —— 站点 location 可选的缓存 zone 名
pub async fn nginx_global_zones(claims: ValidatedClaims) -> ZapJsonResult {
    require_admin(&claims)?;
    let (spec, _) = load_or_seed_spec().await?;
    let zones: Vec<String> = spec
        .get("http")
        .and_then(|h| h.get("proxy_cache"))
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|z| z.get("name").and_then(|n| n.as_str()).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    Ok(Json(
        json!({ "code": 0, "message": "ok", "data": { "zones": zones } }),
    ))
}
