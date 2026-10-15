// SPDX-License-Identifier: AGPL-3.0-only
//! 纯文件持久化存储（替代部分 SQLite 表）。

/// 统一的「面板托管配置」文件存储接口：`data/conf/<project>/`。
pub mod conf;
/// Nginx 主配置可视化：模型与历史版本（统一接口的消费者）。
pub mod nginx_conf;
