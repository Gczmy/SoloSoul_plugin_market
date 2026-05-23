# SoloSoul Plugin Market

> SoloSoul（独灵）官方插件市场
>
> 本仓库以 Git Submodule 形式挂载于主项目 `SoloSoul_code/SoloSoul_plugin_market/`，
> 同时作为独立的 CDN 分发源，供 SoloSoul 主软件运行时动态拉取插件。

---

## 1. 项目概述

SoloSoul Plugin Market 是独灵生态的官方插件仓库，遵循以下核心原则：

- **与主软件生命周期分离**：插件不随 App 二进制打包，主软件通过 CDN 动态下载安装
- **安全优先**：所有插件经 SHA-256 白名单校验 + 版本签名验证后方可加载
- **字段级权限**：插件通过声明式清单请求数据，用户逐字段授权
- **Wasm 沙盒执行**：插件编译为 `wasm32-wasi` 目标，在 Wasmtime 沙盒中隔离运行

---

## 2. 快速开始（插件开发者）

```bash
# 1. 克隆仓库（含子模块）
git clone --recursive git@github.com:Gczmy/SoloSoul_code.git

# 2. 进入插件市场目录
cd SoloSoul_code/SoloSoul_plugin_market

# 3. 安装 wasm32-wasi 目标（如未安装）
rustup target add wasm32-wasi

# 4. 创建新插件
cd plugins
cargo new --lib com.example.my-plugin

# 5. 在 Cargo.toml 中依赖 SDK
# [dependencies]
# solosoul-plugin-sdk = { path = "../../SDK/rust" }

# 6. 编写插件逻辑，编译
cargo build --target wasm32-wasi --release

# 7. 产物位于 target/wasm32-wasi/release/libmy_plugin.wasm
#    重命名为 plugin.wasm 并连同 manifest.json 提交 PR
```

---

## 3. 目录结构

```
SoloSoul_plugin_market/
├── README.md                          # 本文档
├── docs/
│   └── plugin-ideas.md                # 插件创意库（官方/第三方插件开发参考）
├── SDK/                               # 插件开发 SDK
│   ├── rust/                          # Rust SDK（Host Functions 绑定）
│   ├── typescript/                    # AssemblyScript SDK（预留）
│   └── schema/
│       └── manifest.schema.json       # manifest.json JSON Schema
├── registry.json                      # 官方插件白名单注册表
├── plugins/                           # 官方插件源码仓库
│   └── com.solosoul.slotgo/
│       ├── manifest.json
│       ├── plugin.wasm
│       └── src/
│           └── lib.rs
└── examples/                          # 示例插件
    └── hello_world/
```

---

## 4. 插件清单规范（manifest.json）

每个插件必须包含 `manifest.json`，位于插件根目录（与 `plugin.wasm` 同级）：

```json
{
  "plugin_id": "com.solosoul.slotgo",
  "name": "SlotGo - UK Visa Booking",
  "version": "1.0.0",
  "plugin_api_version": "1.0",
  "min_app_version": "1.0.0",
  "max_app_version": "2.0.0",
  "description": "自动监控并预约 UK Visa 面签时间",
  "publisher": "SoloSoul Team",
  "homepage": "https://github.com/Gczmy/SoloSoul_plugin_market",
  "signature": "base64-encoded-ed25519-signature",
  "required_fields": [
    "identity.full_name",
    "travel.primary_passport.number"
  ],
  "optional_fields": [
    "identity.contact.emails"
  ],
  "network_policy": {
    "allowed_domains": ["*.visaservices.com"],
    "block_all_outbound": true
  },
  "data_ttl_seconds": 300,
  "require_user_confirmation": true,
  "consent_validity_hours": 24
}
```

### 字段说明

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `plugin_id` | string | ✅ | 反向域名格式，全局唯一标识 |
| `name` | string | ✅ | 插件显示名称 |
| `version` | string | ✅ | SemVer 格式，如 `1.0.0` |
| `plugin_api_version` | string | ✅ | 插件 ABI 版本，与主软件 `PLUGIN_API_VERSION` 严格匹配 |
| `min_app_version` | string | ✅ | 兼容的最低 SoloSoul App 版本 |
| `max_app_version` | string | ✅ | 兼容的最高 SoloSoul App 版本 |
| `description` | string | ✅ | 一句话描述插件功能 |
| `publisher` | string | ✅ | 发布者名称 |
| `homepage` | string | ❌ | 项目主页 URL |
| `signature` | string | ✅ | Ed25519 签名（Release 模式强制校验） |
| `required_fields` | string[] | ✅ | 插件必需的字段路径列表 |
| `optional_fields` | string[] | ❌ | 可选字段路径列表 |
| `network_policy` | object | ❌ | 网络白名单策略 |
| `data_ttl_seconds` | number | ❌ | 敏感数据内存存活时间，默认 300 |
| `require_user_confirmation` | boolean | ❌ | 是否要求用户确认，默认 `true` |
| `consent_validity_hours` | number | ❌ | 授权有效期，默认 24 |

---

## 5. 插件注册表规范（registry.json）

`registry.json` 是插件市场的索引文件，由 CI/CD 自动维护，供 SoloSoul 主软件拉取：

```json
{
  "version": "1",
  "updated_at": "2026-05-22T00:00:00Z",
  "plugins": {
    "com.solosoul.slotgo": {
      "name": "SlotGo - UK Visa Booking",
      "publisher": "SoloSoul Team",
      "latest_version": "1.0.0",
      "versions": {
        "1.0.0": {
          "sha256": "a3b5c8d7e9f0123456789abcdef0123456789abcdef0123456789abcdef0123",
          "plugin_api_version": "1.0",
          "min_app_version": "1.0.0",
          "max_app_version": "2.0.0",
          "download_url": "https://plugins.solosoul.dev/com.solosoul.slotgo/1.0.0/",
          "released_at": "2026-05-20T00:00:00Z"
        }
      }
    }
  }
}
```

## 6. CI/CD 发布流程

插件通过 GitHub Actions 自动构建并发布到 CDN：

```yaml
# .github/workflows/plugin_release.yml
name: Plugin Release

on:
  push:
    branches: [main]
    paths: ['SoloSoul_plugin_market/**']

jobs:
  build-and-deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: recursive

      - name: Build plugins
        run: |
          cd SoloSoul_plugin_market
          for dir in plugins/*/; do
            cd "$dir"
            rustup target add wasm32-wasi
            cargo build --target wasm32-wasi --release
            cd -
          done

      - name: Upload to CDN
        run: |
          aws s3 sync SoloSoul_plugin_market/plugins/ s3://plugins.solosoul.dev/
          aws s3 cp SoloSoul_plugin_market/registry.json s3://plugins.solosoul.dev/registry.json
```

---

## 7. SDK 使用指南

### Rust SDK

```rust
use solosoul_plugin_sdk::{get_field, post_json};

#[no_mangle]
pub extern "C" fn run() -> i32 {
    let name = match get_field("identity.full_name") {
        Ok(v) => v,
        Err(e) => {
            solosoul_plugin_sdk::log_error(&format!("获取失败: {:?}", e));
            return -1;
        }
    };
    // ... 业务逻辑
    0
}
```

### Host Functions ABI

| 函数 | 返回值 | 说明 |
|------|--------|------|
| `solosoul_request_field(...)` | `i32` | 请求用户字段，0=成功，-1=权限不足，-2=用户拒绝 |
| `solosoul_post_data(...)` | `i32` | 代理网络请求，0=成功，-10=域名未授权 |
| `solosoul_log(...)` | void | 写审计日志 |
| `solosoul_get_timestamp()` | `i64` | 获取 Unix 时间戳（毫秒） |

完整 ABI 规范见主项目文档：`SoloSoul_code/docs/PLUGIN_SYSTEM_DESIGN.md`

---

## 8. 贡献指南

1. Fork 本仓库
2. 在 `plugins/` 下创建新的插件目录（反向域名格式）
3. 编写源码 + `manifest.json`，确保通过 `cargo clippy` 和 `cargo test`
4. 提交 PR，CI 将自动构建并更新 `registry.json`
5. 维护者审核通过后合并，插件自动发布到 CDN

---

*本文档与主项目 `docs/PLUGIN_SYSTEM_DESIGN.md` 同步维护。*
