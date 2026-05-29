# SoloSoul Plugin Market

> SoloSoul（独灵）官方插件市场
>
> 本仓库作为**独立的 Git 公开仓库**，既是插件源码托管中心，也是 SoloSoul 主软件的**动态分发源**。
> 主软件通过 jsDelivr CDN + GitHub Raw fallback 直接从本仓库拉取插件，无需额外服务器。

---

## 1. 项目概述

SoloSoul Plugin Market 是独灵生态的官方插件仓库，遵循以下核心原则：

- **仓库即市场**：GitHub 公开仓库直接充当插件市场，本地生成索引后 push 即发布
- **零服务器成本**：通过 jsDelivr CDN 免费分发，GitHub Raw 作为 fallback
- **安全优先**：所有插件经 SHA-256 白名单校验 + 版本签名验证后方可加载
- **字段级权限**：插件通过声明式清单请求数据，用户逐字段授权
- **Wasm 沙盒执行**：插件编译为 `wasm32-wasip1` 目标，在 Wasmtime 沙盒中隔离运行

### 分发架构

```
┌─────────────────┐     jsDelivr CDN      ┌─────────────────┐
│  SoloSoul 客户端 │  ←──────────────────  │  GitHub 公开仓库  │
│  (Flutter/Rust)  │  fallback: Raw GitHub │  (本仓库)        │
└─────────────────┘                      └─────────────────┘
         │                                          │
         │  1. GET registry.json                    │
         │  2. GET plugins/{id}/plugin.wasm         │
         │  3. GET plugins/{id}/manifest.json       │
         │                                          │
         └──────────── 本地缓存 + SHA-256 校验 ───────┘
```

---

## 2. 快速开始（插件开发者）

```bash
# 1. 克隆本仓库并安装 Git Hooks
git clone git@github.com:Gczmy/SoloSoul_plugin_market.git
cd SoloSoul_plugin_market
bash scripts/install-hooks.sh      # 启用 pre-commit 自动生成 registry

# 2. 安装 wasm32-wasip1 目标（如未安装）
rustup target add wasm32-wasip1

# 3. 创建新插件
cd plugins
mkdir com.example.my-plugin
cd com.example.my-plugin
cargo init --lib

# 4. 在 Cargo.toml 中依赖 SDK
# [dependencies]
# solosoul-plugin-sdk = { path = "../../SDK/rust" }

# 5. 编写插件逻辑 + manifest.json，编译
cargo build --target wasm32-wasip1 --release

# 6. 产物位于 target/wasm32-wasip1/release/*.wasm
#    复制为 plugin.wasm
cp target/wasm32-wasip1/release/*.wasm plugin.wasm

# 7. 提交（pre-commit hook 会自动重新生成 registry.json）
cd ../..                           # 回到仓库根目录
git add -A
git commit -m "feat: add my-plugin v1.0.0"
git push origin main
```

**完成！** push 后 CI 会验证 `registry.json` 与 `plugins/` 目录一致，验证通过后客户端即可发现新插件。

---

## 3. 目录结构

```
SoloSoul_plugin_market/
├── README.md                          # 本文档
├── registry.json                      # 插件索引（本地生成，随代码提交）
├── .github/
│   └── workflows/
│       ├── validate-registry.yml      # CI：push 时验证 registry.json 与 plugins/ 一致
│       └── update-registry.yml        # CI：手动触发，紧急重建 registry（兜底）
├── scripts/
│   └── generate_registry.py           # registry.json 生成脚本
├── docs/
│   └── plugin-ideas.md                # 插件创意库
├── SDK/                               # 插件开发 SDK
│   ├── rust/                          # Rust SDK（Host Functions 绑定）
│   ├── typescript/                    # AssemblyScript SDK（预留）
│   └── schema/
│       └── manifest.schema.json       # manifest.json JSON Schema
├── plugins/                           # 官方插件源码 + wasm
│   └── com.solosoul.official.id-validator/
│       ├── manifest.json              # 插件清单
│       ├── plugin.wasm                # 编译产物
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs
└── examples/                          # 示例插件
    └── hello_world/
```

---

## 4. Git Hooks 安装（推荐）

安装 pre-commit hook 后，提交插件变更时会**自动重新生成 `registry.json`**，无需手动记忆。

```bash
# 在仓库根目录执行
bash scripts/install-hooks.sh
```

此脚本会：
- 配置 Git 使用 `.githooks/` 目录
- 检查 `python3` 可用性

安装后，每次 `git commit` 若检测到 `plugins/` 有变更，hook 会自动运行 `generate_registry.py` 并将更新后的 `registry.json` 加入当前提交。

> 如环境缺少 Python 3，hook 会友好跳过并给出提示，不会阻塞提交。
> 如需临时跳过 hook：`git commit --no-verify`

---

## 5. 插件清单规范（manifest.json）

每个插件必须包含 `manifest.json`，位于插件根目录（与 `plugin.wasm` 同级）：

```json
{
  "plugin_id": "com.solosoul.official.id-validator",
  "name": "ID Validator",
  "version": "1.0.0",
  "plugin_api_version": "1.0",
  "min_app_version": "1.0.0",
  "max_app_version": "999.999.999",
  "description": "中国居民身份证 18 位校验",
  "publisher": "SoloSoul Official",
  "homepage": "https://github.com/Gczmy/SoloSoul_plugin_market/tree/main/plugins/com.solosoul.official.id-validator",
  "required_fields": [
    "idCard.number"
  ],
  "optional_fields": [],
  "network_policy": {
    "block_all_outbound": true
  },
  "data_ttl_seconds": 60,
  "require_user_confirmation": false
}
```

### 字段说明

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `plugin_id` | string | ✅ | 反向域名格式，全局唯一标识 |
| `name` | string | ✅ | 插件显示名称 |
| `version` | string | ✅ | SemVer 格式，如 `1.0.0` |
| `plugin_api_version` | string | ✅ | 插件 ABI 版本，与主软件严格匹配 |
| `min_app_version` | string | ✅ | 兼容的最低 SoloSoul App 版本 |
| `max_app_version` | string | ✅ | 兼容的最高 SoloSoul App 版本 |
| `description` | string | ✅ | 一句话描述插件功能 |
| `publisher` | string | ✅ | 发布者名称 |
| `homepage` | string | ❌ | 项目主页 URL |
| `required_fields` | string[] | ✅ | 插件必需的字段路径列表 |
| `optional_fields` | string[] | ❌ | 可选字段路径列表 |
| `network_policy` | object | ❌ | 网络白名单策略 |
| `data_ttl_seconds` | number | ❌ | 敏感数据内存存活时间，默认 300 |
| `require_user_confirmation` | boolean | ❌ | 是否要求用户确认，默认 `true` |

---

## 6. 插件注册表规范（registry.json）

> **此文件由开发者本地生成并随代码提交。**
> 修改插件后，请在仓库根目录运行 `python3 scripts/generate_registry.py` 更新此文件。
> 推荐安装 Git Hooks（`bash scripts/install-hooks.sh`）以自动完成此步骤。

`registry.json` 是插件市场的机器可读索引，供 SoloSoul 客户端拉取：

```json
{
  "version": "1",
  "updated_at": "2026-05-24T00:00:00Z",
  "plugins": {
    "com.solosoul.official.id-validator": {
      "name": "ID Validator",
      "publisher": "SoloSoul Official",
      "latest_version": "1.0.0",
      "versions": {
        "1.0.0": {
          "sha256": "e2ee0a3e98eb013a20ab1e77d3e17bd8ced5941a6400ae810940b9046c8e6e0f",
          "plugin_api_version": "1.0",
          "min_app_version": "1.0.0",
          "max_app_version": "999.999.999",
          "download_url": "https://cdn.jsdelivr.net/gh/Gczmy/SoloSoul_plugin_market@main/plugins/com.solosoul.official.id-validator/plugin.wasm",
          "raw_url": "https://raw.githubusercontent.com/Gczmy/SoloSoul_plugin_market/main/plugins/com.solosoul.official.id-validator/plugin.wasm",
          "released_at": "2026-05-24T00:00:00Z"
        }
      }
    }
  }
}
```

### 字段说明

| 字段 | 说明 |
|------|------|
| `download_url` | **优先下载地址**，jsDelivr CDN（中国大陆访问友好） |
| `raw_url` | **fallback 地址**，GitHub Raw 直连 |
| `sha256` | `plugin.wasm` 的 SHA-256 哈希，客户端安装时强制校验 |

### 下载策略

SoloSoul 客户端按以下优先级下载：

1. `download_url`（jsDelivr CDN）— 全球加速，国内可用
2. `raw_url`（GitHub Raw）— CDN 失败时 fallback
3. 内置 `assets/registry.json` — 完全离线时的兜底

---

## 6. CI/CD 发布流程

本仓库采用 **本地预生成 + CI 验证** 模式：

### 触发条件

- **`push` / `pull_request` → `validate-registry.yml`**：自动验证 `registry.json` 是否与 `plugins/` 目录一致
- **`workflow_dispatch` → `update-registry.yml`**：手动触发，紧急重建 `registry.json`（仅维护者）

### 执行流程

```
开发者本地：                    GitHub CI (push)：
1. 修改 plugins/xxx/            1. checkout 仓库
2. 运行 generate_registry.py    2. 运行 generate_registry.py
3. git add -A                   3. git diff 对比 registry.json
4. git commit & push            4. 不一致 → ❌ CI 失败，阻止合并
                                5. 一致   → ✅ CI 通过
```

### 本地生成 registry.json

```bash
# 在仓库根目录执行
python3 scripts/generate_registry.py
# 输出：Generated registry.json with N plugin(s)

# 安装 Git Hooks 后，提交时会自动生成（推荐）
bash scripts/install-hooks.sh
```

环境变量（用于覆盖默认仓库地址）：

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `GITHUB_OWNER` | `Gczmy` | GitHub 仓库所有者 |
| `GITHUB_REPO` | `SoloSoul_plugin_market` | 仓库名称 |
| `GITHUB_BRANCH` | `main` | 分支名 |

---

## 7. 插件版本更新流程

更新现有插件无需创建 Release Tag，修改源码后本地生成 registry 并 push：

```bash
cd plugins/com.solosoul.official.my-plugin

# 1. 修改源码
# 2. 更新 manifest.json 中的 version 字段
# 3. 重新编译
rustup run stable cargo build --target wasm32-wasip1 --release
cp target/wasm32-wasip1/release/*.wasm plugin.wasm

# 4. 回到仓库根目录，重新生成 registry.json（已安装 hooks 则自动完成）
cd ../..
python3 scripts/generate_registry.py

# 5. 提交（若已安装 hooks，registry.json 会自动加入提交）
git add -A
git commit -m "feat(id-validator): add visa expiry check v1.1.0"
git push origin main

# 6. CI 验证 registry.json 与 plugins/ 一致后通过
```

> **注意**：旧版本不会从 registry 中删除，客户端仍可选择安装历史版本（未来功能）。

### CI 失败修复指南

如果 `validate-registry.yml` 报告 `registry.json` 不一致，在本地执行：

```bash
python3 scripts/generate_registry.py
git add registry.json

# 方式一：修正当前 commit（推荐，PR 分支）
git commit --amend --no-edit
git push --force-with-lease

# 方式二：新增一个修复 commit
git commit -m "chore: update registry.json"
git push
```

### 多人协作冲突处理

多人同时修改不同插件时，`registry.json` 可能产生合并冲突。处理方式：

1. **预防**：GitHub 仓库建议开启 "Require branches to be up to date before merging"，确保 PR 合并前已 rebase 到最新 `main`
2. **解决冲突**：与普通代码冲突一致 —— 在本地 `git merge origin/main` 后重新运行 `generate_registry.py`，提交更新后的 `registry.json`
3. **PR 修正**：若 CI 因 registry 不一致失败，参见上方 "CI 失败修复指南"

---

## 8. SDK 使用指南

### Rust SDK

```rust
use solosoul_plugin_sdk::{get_field, log_info, log_error};

#[no_mangle]
pub extern "C" fn run() -> i32 {
    match get_field("idCard.number") {
        Ok(value) => {
            log_info(&format!("读取成功: {}", value));
            // ... 业务逻辑
            0
        }
        Err(e) => {
            log_error(&format!("获取失败: {:?}", e));
            -1
        }
    }
}
```

### Host Functions ABI

| 函数 | 返回值 | 说明 |
|------|--------|------|
| `solosoul_request_field(...)` | `i32` | 请求用户字段，0=成功，负数=错误码 |
| `solosoul_post_data(...)` | `i32` | 代理网络请求，0=成功，-10=域名未授权 |
| `solosoul_log(...)` | void | 写审计日志 |
| `solosoul_get_timestamp()` | `i64` | 获取 Unix 时间戳（毫秒） |

完整 ABI 规范见主项目文档：`SoloSoul_code/docs/PLUGIN_SYSTEM_DESIGN.md`

---

## 9. 贡献指南

1. Fork 本仓库
2. 在 `plugins/` 下创建新的插件目录（反向域名格式）
3. 编写源码 + `manifest.json`，确保通过 `cargo clippy` 和 `cargo test`
4. 编译为 `wasm32-wasip1`，生成 `plugin.wasm`
5. 本地运行 `python3 scripts/generate_registry.py` 更新 `registry.json`
6. 提交 PR，CI 将验证 `registry.json` 与 `plugins/` 一致性
7. 维护者审核通过后合并，插件即刻上线

### 第三方插件市场

任何公开 GitHub 仓库都可以作为 SoloSoul 的插件源，只需满足：

- 仓库根目录包含 `registry.json`
- 插件目录结构为 `plugins/{plugin_id}/{manifest.json, plugin.wasm}`

用户可在 SoloSoul 客户端设置中添加自定义源：

```
源名称: 我的私有市场
仓库: myuser/my-plugins
分支: main
CDN 加速: 是
```

---

*本文档与主项目 `docs/PLUGIN_SYSTEM_DESIGN.md` 同步维护。*
