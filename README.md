# SoloSoul Plugin Market

> SoloSoul（独灵）官方插件市场 —— Wasm 沙盒插件的源码仓库与动态分发源

[![Validate Registry](https://github.com/Gczmy/SoloSoul_plugin_market/actions/workflows/validate-registry.yml/badge.svg)](https://github.com/Gczmy/SoloSoul_plugin_market/actions/workflows/validate-registry.yml)

SoloSoul 插件系统为个人数字孪生引擎提供可扩展能力：证件校验、TOTP 双因素、地址/电话格式化、MRZ 编码、附件水印…… 插件以 **WebAssembly** 形式编译，在 SoloSoul 客户端（Tauri，Rust + React）内置的 **Wasmtime 沙盒**中隔离运行。所有数据处理都在用户本机完成，网络请求经宿主白名单代理，敏感字段按「字段级授权」逐项放行。

- **零服务器分发**：本仓库即市场。本地生成索引 `registry.json` 后 push 即发布，客户端经 CDN 拉取，无需自建服务器
- **沙盒隔离**：Wasmtime + WASI Preview1，燃料上限 + 编译缓存，移动端自动切换 Pulley 解释器
- **可验证**：插件安装强制 SHA-256 校验；注册表 minisign 签名验证
- **最小授权**：`required_fields` / `optional_fields` 声明字段范围，用户逐字段授权（Consent），数据 TTL 到期即清

---

## 目录

- [架构总览](#架构总览)
- [官方插件](#官方插件)
- [快速开始（插件开发者）](#快速开始插件开发者)
- [目录结构](#目录结构)
- [插件清单规范（manifest.json）](#插件清单规范manifestjson)
- [插件注册表规范（registry.json）](#插件注册表规范registryjson)
- [SDK（Rust）](#sdkrust)
- [发布与更新插件](#发布与更新插件)
- [贡献指南](#贡献指南)
- [相关文档](#相关文档)

---

## 架构总览

```
┌───────────────────────────────  SoloSoul 客户端（Tauri: React + Rust）  ───────────────────────────────┐
│                                                                                                         │
│  前端插件看板 PluginDashboardPage（安装 / 更新 / 卸载 / 运行 / 审计日志）                                  │
│        │  Tauri IPC（plugin_install / plugin_run / plugin_consent_response …）                          │
│        ▼                                                                                                │
│  Rust 宿主  solosoul-plugin crate                                                                        │
│  ├─ PluginManager   安装、更新、卸载、运行编排                                                           │
│  ├─ PluginRegistry  注册表加载与更新（minisign 验签）                                                    │
│  ├─ WasmSandbox     Wasmtime 沙盒（WASI Preview1 + 燃料上限 + 编译缓存）                                 │
│  ├─ SoloHostFunctions  Host Functions（字段读取 / 网络代理 / 结果回传 / 审计）                            │
│  ├─ ConsentManager  字段级授权 + 数据 TTL                                                               │
│  └─ RateLimiter / AuditLogger                                                                           │
│                                                                                                         │
│  本地数据：Vault（加密存储） ── 插件只能通过 Host Functions 访问，无法直接触达                                  │
└─────────────────────────────────────────────────────────────────────────────────────────────────────────┘
        │ ① 拉取注册表（minisign 验签）
        │ ② 下载 plugin.wasm（SHA-256 校验）
        ▼
┌───────────────────────────────────────────────  本仓库  ───────────────────────────────────────────────┐
│  plugins/{plugin_id}/  插件源码 + manifest.json + plugin.wasm（提交即发布）                                │
│  registry.json         插件索引（scripts/generate_registry.py 本地生成）                                  │
│  SDK/rust              Rust SDK（solosoul-plugin-sdk）                                                  │
└─────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### 客户端消费链路

1. **注册表**：启动时从 `https://plugins.solosoul.app/registry.json` 拉取（可用环境变量 `SOLOSOUL_REGISTRY_URL` 覆盖），用 `SOLOSOUL_REGISTRY_PUBKEY` 对应私钥的 **minisign 签名**校验完整性；未配置公钥或拉取失败时，回落使用应用内置的 bundled 注册表。
2. **插件二进制**：按注册表条目的 `download_url`（jsDelivr CDN）下载，失败回退 `raw_url`（GitHub Raw）；再失败则尝试随应用分发的本地副本。
3. **安装校验**：下载完成后计算 SHA-256 并与注册表记录的 `sha256` 强制比对，不一致即拒绝安装。
4. **运行**：插件在 Wasmtime 沙盒中执行——WASI Preview1、单次运行 100 亿燃料上限、stdio 静默丢弃（插件无法向宿主注入日志）；桌面端 Cranelift JIT，Android / iOS 自动切换 Pulley 解释器。同一 wasm 的编译产物以 SHA-256 为键进程级缓存。

---

## 官方插件

共 **21 个**官方插件（`tier` 表示分批启用层级，p0 最高优先）：

| 插件 | 版本 | Tier | 说明 |
|------|------|------|------|
| `com.solosoul.official.address-fmt` | 1.0.5 | p0 | 地址格式化器——按目标国家/地区规范格式化地址 |
| `com.solosoul.official.calendar-events` | 1.0.2 | p0 | 日历事件生成器——证件到期日生成 iCalendar 提醒 |
| `com.solosoul.official.contact-exporter` | 1.0.2 | p0 | 联系人导出器——联系信息导出为 CSV |
| `com.solosoul.official.data-completeness` | 1.0.2 | p2 | 档案完整度扫描——检查各分区完成度并给出建议 |
| `com.solosoul.official.digital-will` | 1.0.1 | p2 | 数字遗产指示——紧急情况下的资产与账户处理建议 |
| `com.solosoul.official.doc-checklist` | 1.0.2 | p0 | 材料清单检查器——签证/旅行/银行场景材料检查 |
| `com.solosoul.official.emergency-card` | 1.0.1 | p2 | 紧急联系卡——医疗信息与紧急联系人应急卡片 |
| `com.solosoul.official.expiry-guardian` | 1.1.0 | p0 | 证件到期卫士——基于契约扫描证件有效期并分级预警 |
| `com.solosoul.official.form-prefiller` | 1.0.3 | p2 | 表单预填助手——生成 Vault 字段映射表 |
| `com.solosoul.official.id-validator` | 1.0.2 | p0 | 证件校验——身份证校验位与护照格式检查 |
| `com.solosoul.official.identity-timeline` | 1.0.1 | p0 | 身份时间线——教育、工作、证件等人生里程碑 |
| `com.solosoul.official.mrz-encoder` | 1.0.2 | p0 | MRZ 编码器——护照信息编码为 ICAO Doc 9303 TD3 |
| `com.solosoul.official.namecard-gen` | 1.0.1 | p0 | 数字名片生成器——生成 vCard 3.0 数字名片 |
| `com.solosoul.official.packing-list` | 1.0.1 | p0 | 行李清单生成器——按目的地与季节推荐打包清单 |
| `com.solosoul.official.phone-fmt` | 1.0.2 | p0 | 电话格式化器——按国家规范格式化电话号码 |
| `com.solosoul.official.resume-builder` | 1.0.1 | p2 | 简历生成器——从 Vault 档案生成 Markdown 简历 |
| `com.solosoul.official.tax-profile` | 1.0.1 | p2 | 税务档案摘要——汇总税务居民身份与申报基础数据 |
| `com.solosoul.official.totp-gen` | 1.0.2 | p0 | TOTP 生成器——基于 2FA Secret 生成动态验证码 |
| `com.solosoul.official.travel-footprint` | 1.0.2 | p0 | 旅行足迹分析——到访国家统计与旅行报告 |
| `com.solosoul.official.watermark` | 1.0.0 | p0 | 附件水印——图片/PDF 批量添加文本水印（仅作用于副本） |
| `com.solosoul.official.slotgo` | 0.1.0 | p1 | UK Visa 预约时间查询助手 |

---

## 快速开始（插件开发者）

```bash
# 1. 克隆本仓库并安装 Git Hooks（自动生成 registry.json）
git clone git@github.com:Gczmy/SoloSoul_plugin_market.git
cd SoloSoul_plugin_market
bash scripts/install-hooks.sh

# 2. 安装 wasm32-wasip1 目标（如未安装）
rustup target add wasm32-wasip1

# 3. 创建新插件
cd plugins
mkdir com.example.my-plugin
cd com.example.my-plugin
cargo init --lib
```

在 `Cargo.toml` 中依赖 SDK：

```toml
[dependencies]
solosoul-plugin-sdk = { path = "../../SDK/rust" }

[lib]
crate-type = ["cdylib"]
```

编写插件逻辑与 `manifest.json`，然后编译：

```bash
cargo build --target wasm32-wasip1 --release
cp target/wasm32-wasip1/release/*.wasm plugin.wasm
```

提交（pre-commit hook 会自动重新生成 `registry.json`）：

```bash
cd ../..
git add -A
git commit -m "feat: add my-plugin v1.0.0"
git push origin main
```

push 后 CI（`validate-registry.yml`）会验证 `registry.json` 与 `plugins/` 目录一致，通过后客户端即可发现新插件。

> 最小可运行示例见 [`examples/hello_world`](examples/hello_world)。

---

## 目录结构

```
SoloSoul_plugin_market/
├── README.md                          # 本文档
├── registry.json                      # 插件索引（本地生成，随代码提交）
├── .githooks/
│   └── pre-commit                     # 提交时自动重新生成 registry.json
├── .github/workflows/
│   ├── validate-registry.yml          # CI：push/PR 时验证 registry.json 与 plugins/ 一致
│   └── update-registry.yml            # CI：手动触发，紧急重建 registry（兜底）
├── scripts/
│   ├── generate_registry.py           # registry.json 生成脚本
│   └── install-hooks.sh               # Git Hooks 安装脚本
├── docs/
│   └── plugin-ideas.md                # 插件创意库
├── SDK/
│   ├── rust/                          # Rust SDK（solosoul-plugin-sdk，Host Functions 绑定）
│   └── schema/
│       └── manifest.schema.json       # manifest.json JSON Schema（draft-07）
├── plugins/                           # 官方插件源码 + 编译产物
│   └── com.solosoul.official.<name>/
│       ├── manifest.json              # 插件清单
│       ├── plugin.wasm                # 编译产物（wasm32-wasip1）
│       ├── Cargo.toml
│       └── src/lib.rs
└── examples/
    └── hello_world/                   # 最小示例插件
```

---

## 插件清单规范（manifest.json）

每个插件根目录（与 `plugin.wasm` 同级）必须包含 `manifest.json`：

```json
{
  "plugin_id": "com.solosoul.official.id-validator",
  "name": "ID Validator",
  "version": "1.0.2",
  "plugin_api_version": "1.0",
  "min_app_version": "1.0.0",
  "max_app_version": "999.999.999",
  "description": "校验证件号码格式与校验位 — 支持中国居民身份证校验与护照格式检查",
  "publisher": "SoloSoul Official",
  "homepage": "https://github.com/Gczmy/SoloSoul_plugin_market/tree/main/plugins/com.solosoul.official.id-validator",
  "required_fields": [],
  "optional_fields": [
    "idCard.number",
    "passport.number"
  ],
  "network_policy": {
    "block_all_outbound": true
  },
  "data_ttl_seconds": 60,
  "require_user_confirmation": false,
  "i18n": {
    "zh": { "name": "证件校验", "description": "校验各国证件号码格式与校验位" },
    "en": { "name": "ID Validator", "description": "Validate ID number formats and check digits" }
  },
  "tier": "p0",
  "category": "validator"
}
```

### 字段说明

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `plugin_id` | string | ✅ | 反向域名格式，全局唯一标识 |
| `name` | string | ✅ | 插件显示名称 |
| `version` | string | ✅ | SemVer 格式，如 `1.0.2` |
| `plugin_api_version` | string | ✅ | 插件 ABI 版本，与客户端严格匹配（当前官方插件 1.0–2.0） |
| `min_app_version` | string | ✅ | 兼容的最低 SoloSoul 客户端版本 |
| `max_app_version` | string | ✅ | 兼容的最高 SoloSoul 客户端版本 |
| `description` | string | ✅ | 一句话描述插件功能 |
| `publisher` | string | ✅ | 发布者名称 |
| `homepage` | string | ❌ | 项目主页 URL |
| `required_fields` | string[] | ✅ | 必需字段路径列表（缺失时客户端提示） |
| `optional_fields` | string[] | ❌ | 可选字段路径列表 |
| `network_policy` | object | ❌ | 网络策略：`block_all_outbound`（默认 `true`）+ `allowed_domains` 白名单 |
| `data_ttl_seconds` | number | ❌ | 授权数据内存存活时间（秒），默认 `300` |
| `require_user_confirmation` | boolean | ❌ | 是否每次运行都要求用户确认，默认 `true` |
| `i18n` | object | ❌ | `zh` / `en` 等语言的插件名称与描述覆盖 |
| `tier` | string | ❌ | 分批启用层级：`p0`–`p4`（默认 `p3`） |
| `category` | string | ❌ | 插件分类标识（如 `validator`、`formatter`） |
| `contracts` | array | ❌ | 类型契约：`typeId`、`version`、`typeIdAliases`、`roles`（语义槽位）、`bindings` |
| `params` | array | ❌ | 运行参数声明（`key`、`label`、`type`、`options`），运行时由用户填写 |

> 完整 JSON Schema 见 [`SDK/schema/manifest.schema.json`](SDK/schema/manifest.schema.json)。

---

## 插件注册表规范（registry.json）

> **此文件由开发者本地生成并随代码提交**，客户端不依赖构建时生成。
> 修改插件后运行 `python3 scripts/generate_registry.py`，或安装 Git Hooks 自动完成。

```json
{
  "version": "1",
  "updated_at": "2026-07-04T00:56:25Z",
  "plugins": {
    "com.solosoul.official.id-validator": {
      "name": "ID Validator",
      "publisher": "SoloSoul Official",
      "latest_version": "1.0.2",
      "versions": {
        "1.0.2": {
          "sha256": "e2ee0a3e98eb013a20ab1e77d3e17bd8ced5941a6400ae810940b9046c8e6e0f",
          "plugin_api_version": "1.0",
          "min_app_version": "1.0.0",
          "max_app_version": "999.999.999",
          "download_url": "https://cdn.jsdelivr.net/gh/Gczmy/SoloSoul_plugin_market@main/plugins/com.solosoul.official.id-validator/plugin.wasm",
          "raw_url": "https://raw.githubusercontent.com/Gczmy/SoloSoul_plugin_market/main/plugins/com.solosoul.official.id-validator/plugin.wasm",
          "released_at": "2026-07-04T00:56:25Z"
        }
      }
    }
  }
}
```

### 字段说明

| 字段 | 说明 |
|------|------|
| `sha256` | `plugin.wasm` 的 SHA-256 哈希，客户端**安装时强制校验** |
| `download_url` | 首选下载地址（jsDelivr CDN，中国大陆访问友好） |
| `raw_url` | fallback 地址（GitHub Raw 直连） |
| `plugin_api_version` / `min_app_version` / `max_app_version` | 客户端据此做兼容性过滤 |

---

## SDK（Rust）

`SDK/rust` 提供类型安全的 Host Functions 绑定（crate 名 `solosoul-plugin-sdk`）。插件只需：

```rust
use solosoul_plugin_sdk::{get_field, log_info, send_result_json};

#[no_mangle]
pub extern "C" fn run() -> i32 {
    match get_field("idCard.number") {
        Ok(value) => {
            log_info(&format!("读取成功: {}", value));
            send_result_json(&format!("{{\"valid\": true, \"number\": \"{}\"}}", value))
                .unwrap_or(1);
            0
        }
        Err(e) => {
            log_info(&format!("读取失败: {:?}", e));
            1
        }
    }
}
```

> 入口约定：插件必须导出 `#[no_mangle] pub extern "C" fn run() -> i32`（SDK 提供 `PluginMain` 类型别名）。返回 `0` 表示成功，非零为自定义错误码。

### Host Functions（SDK API）

| 分组 | 函数 | 说明 |
|------|------|------|
| 数据访问 | `list_objects(type_id)` | 列出指定契约类型的所有对象（JSON） |
| | `get_field(field_id)` | 请求字段数据（受 Consent 授权约束） |
| | `get_data_structure_tree()` | 获取 Vault 数据契约结构树 |
| | `list_attachments()` / `prepare_attachment_copy(object_id, attachment_id)` | 附件列表与副本准备 |
| 网络 | `post_json(url, json_body)` | 经宿主代理的 HTTP POST（受 `network_policy` 域名白名单约束） |
| 交互 | `show_dialog(config_json)` | 弹出用户交互对话框 |
| 参数 | `get_param(key)` / `get_locale()` | 读取运行参数 / 当前语言 |
| 结果 | `send_result_json(json)` | 回传结构化结果 |
| | `result_text` / `result_key_value` / `result_table` / `result_markdown` | 便捷结果构造 |
| | `write_output_file(file_name, bytes)` / `copy_output_file(src, file_name)` | 生成可导出文件（水印/导出类插件） |
| 工具 | `log` / `log_info` / `log_warn` / `log_error` / `log_debug` | 结构化日志（进入审计流） |
| | `get_timestamp()` / `sleep(ms)` | 时间与休眠（单次最多 1000ms） |
| | `escape_json` / `truncate` / `parse_date_yyyymmdd_or_iso` / `days_until_ymd` | 常用工具函数 |
| 图片/PDF | `image_watermark` / `pdf_watermark` | 附件水印能力 |

> Host Functions 完整 ABI、错误码与主项目衔接见 [`docs/wasm-plugin-development-guide.md`](https://github.com/Gczmy/SoloSoul/blob/main/docs/wasm-plugin-development-guide.md) 与主项目 [`docs/plugin_market/`](https://github.com/Gczmy/SoloSoul/tree/main/docs/plugin_market)。

---

## 发布与更新插件

本仓库采用 **本地预生成 + CI 验证** 模式。

### 更新已有插件

```bash
cd plugins/com.solosoul.official.my-plugin

# 1. 修改源码；2. 更新 manifest.json 的 version 字段
# 3. 重新编译并复制产物
cargo build --target wasm32-wasip1 --release
cp target/wasm32-wasip1/release/*.wasm plugin.wasm

# 4. 回到仓库根目录，重新生成 registry.json（已装 hooks 则自动完成）
cd ../..
python3 scripts/generate_registry.py

# 5. 提交并推送，CI 验证通过后自动上线
git add -A
git commit -m "feat(id-validator): add visa expiry check v1.1.0"
git push origin main
```

无需创建 Release Tag。旧版本会保留在 `registry.json` 的 `versions` 中。

### CI 流程

| 工作流 | 触发 | 作用 |
|--------|------|------|
| `validate-registry.yml` | push / PR（涉及 `plugins/`） | 重新生成 `registry.json` 并与提交版本 diff，不一致则失败 |
| `update-registry.yml` | 手动触发 | 紧急重建 `registry.json`（仅维护者兜底） |

### CI 失败修复

```bash
python3 scripts/generate_registry.py
git add registry.json

# 方式一：修正当前 commit（推荐，PR 分支）
git commit --amend --no-edit
git push --force-with-lease

# 方式二：新增修复 commit
git commit -m "chore: update registry.json"
git push
```

### 生成脚本环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `GITHUB_OWNER` | `Gczmy` | GitHub 仓库所有者 |
| `GITHUB_REPO` | `SoloSoul_plugin_market` | 仓库名称 |
| `GITHUB_BRANCH` | `main` | 分支名 |

---

## 贡献指南

### 新增官方插件

1. 在 `plugins/` 下创建反向域名格式的插件目录
2. 编写源码 + `manifest.json`，字段声明遵循最小授权原则（`required_fields` 只声明真正必需的）
3. `cargo clippy` 与 `cargo test` 通过，编译为 `wasm32-wasip1` 并生成 `plugin.wasm`
4. 本地运行 `python3 scripts/generate_registry.py` 更新 `registry.json`
5. 提交 PR，CI 验证通过后由维护者合并，插件即刻上线

### 第三方插件市场

任何满足以下条件的公开 GitHub 仓库都可以作为 SoloSoul 的插件源：

- 仓库根目录包含 `registry.json`（结构见上文）
- 插件目录结构为 `plugins/{plugin_id}/{manifest.json, plugin.wasm}`

客户端通过环境变量接入自定义源（默认官方源 `https://plugins.solosoul.app/registry.json`）：

| 环境变量 | 说明 |
|----------|------|
| `SOLOSOUL_REGISTRY_URL` | 注册表地址（如私有市场的 `registry.json` URL） |
| `SOLOSOUL_REGISTRY_PUBKEY` | 注册表 minisign 公钥（Base64）——未配置时跳过远程注册表，使用内置 bundled 注册表 |

---

## 相关文档

| 文档 | 位置 | 内容 |
|------|------|------|
| 插件系统架构设计（15 篇） | 主项目 [`docs/plugin_market/`](https://github.com/Gczmy/SoloSoul/tree/main/docs/plugin_market) | 数据流、安全机制、生命周期、Host ABI |
| Wasm 插件开发指南 | 主项目 [`docs/wasm-plugin-development-guide.md`](https://github.com/Gczmy/SoloSoul/blob/main/docs/wasm-plugin-development-guide.md) | ABI 规范、错误码、开发步骤 |
| 插件创意库 | [`docs/plugin-ideas.md`](docs/plugin-ideas.md) | 待开发插件点子 |

---

*本文档与主项目 `docs/plugin_market/` 及 `docs/wasm-plugin-development-guide.md` 同步维护。*
