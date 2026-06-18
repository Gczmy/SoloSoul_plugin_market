#!/usr/bin/env python3
"""
自动生成 registry.json

扫描 plugins/ 目录，读取每个插件的 manifest.json 和 plugin.wasm，
生成包含元数据和下载地址的注册表索引文件。

用法:
    python3 scripts/generate_registry.py

环境变量:
    GITHUB_OWNER   - GitHub 仓库所有者 (默认: Gczmy)
    GITHUB_REPO    - GitHub 仓库名称 (默认: SoloSoul_plugin_market)
    GITHUB_BRANCH  - Git 分支 (默认: main)
"""

import hashlib
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

PLUGINS_DIR = Path("plugins")
REGISTRY_FILE = Path("registry.json")

DEFAULT_OWNER = "Gczmy"
DEFAULT_REPO = "SoloSoul_plugin_market"
DEFAULT_BRANCH = "main"


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def build_jsdelivr_url(owner: str, repo: str, branch: str, plugin_id: str) -> str:
    """jsDelivr CDN 加速链接，中国大陆访问更稳定"""
    return f"https://cdn.jsdelivr.net/gh/{owner}/{repo}@{branch}/plugins/{plugin_id}/plugin.wasm"


def build_raw_url(owner: str, repo: str, branch: str, plugin_id: str) -> str:
    """GitHub Raw 直连链接（fallback）"""
    return f"https://raw.githubusercontent.com/{owner}/{repo}/{branch}/plugins/{plugin_id}/plugin.wasm"


def scan_plugins(owner: str, repo: str, branch: str) -> dict:
    plugins = {}

    if not PLUGINS_DIR.exists():
        print(f"Error: {PLUGINS_DIR} does not exist", file=sys.stderr)
        sys.exit(1)

    for plugin_dir in sorted(PLUGINS_DIR.iterdir()):
        if not plugin_dir.is_dir():
            continue

        manifest_path = plugin_dir / "manifest.json"
        wasm_path = plugin_dir / "plugin.wasm"

        if not manifest_path.exists():
            print(f"Warning: skipping {plugin_dir.name} (no manifest.json)", file=sys.stderr)
            continue

        if not wasm_path.exists():
            print(f"Warning: skipping {plugin_dir.name} (no plugin.wasm)", file=sys.stderr)
            continue

        try:
            with open(manifest_path, "r", encoding="utf-8") as f:
                manifest = json.load(f)
        except json.JSONDecodeError as e:
            print(f"Error: invalid JSON in {manifest_path}: {e}", file=sys.stderr)
            continue

        plugin_id = manifest.get("plugin_id", plugin_dir.name)
        version = manifest.get("version", "0.0.0")
        wasm_sha256 = sha256_file(wasm_path)

        # 以 manifest 中的 plugin_id 为 key
        if plugin_id not in plugins:
            plugins[plugin_id] = {
                "name": manifest.get("name", plugin_id),
                "publisher": manifest.get("publisher", "Unknown"),
                "latest_version": version,
                "versions": {},
                "description": manifest.get("description"),
                "homepage": manifest.get("homepage"),
                "tier": manifest.get("tier", "p3"),
                "category": manifest.get("category", "utility"),
            }
            # 转发 contracts/field_bindings（Stage 4 typed-lookup）
            contracts = manifest.get("contracts")
            if contracts:
                plugins[plugin_id]["contracts"] = contracts
            field_bindings = manifest.get("field_bindings")
            if field_bindings:
                plugins[plugin_id]["field_bindings"] = field_bindings
            # 提取多语言信息
            i18n = manifest.get("i18n")
            if i18n:
                plugins[plugin_id]["i18n"] = i18n

        # 版本号比较，更新 latest_version
        existing_versions = plugins[plugin_id]["versions"]
        if version not in existing_versions:
            # 简单字符串比较（语义化版本通常可按字符串排序）
            if version > plugins[plugin_id]["latest_version"]:
                plugins[plugin_id]["latest_version"] = version

        version_entry = {
            "sha256": wasm_sha256,
            "plugin_api_version": manifest.get("plugin_api_version", "1.0"),
            "min_app_version": manifest.get("min_app_version", "1.0.0"),
            "max_app_version": manifest.get("max_app_version", "999.999.999"),
            "download_url": build_jsdelivr_url(owner, repo, branch, plugin_dir.name),
            "raw_url": build_raw_url(owner, repo, branch, plugin_dir.name),
            "released_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        }
        # 提取版本变更日志
        changelog = manifest.get("changelog")
        if changelog:
            version_entry["changelog"] = changelog
        existing_versions[version] = version_entry

    return plugins


def main():
    owner = os.environ.get("GITHUB_OWNER", DEFAULT_OWNER)
    repo = os.environ.get("GITHUB_REPO", DEFAULT_REPO)
    branch = os.environ.get("GITHUB_BRANCH", DEFAULT_BRANCH)

    plugins = scan_plugins(owner, repo, branch)

    registry = {
        "version": "1",
        "updated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "plugins": plugins,
    }

    # 合并旧 registry：保留历史版本记录 + 保留未变更版本的 released_at
    old_registry = None
    if REGISTRY_FILE.exists():
        try:
            with open(REGISTRY_FILE, "r", encoding="utf-8") as f:
                old_registry = json.load(f)
            for plugin_id, plugin_entry in old_registry.get("plugins", {}).items():
                if plugin_id not in plugins:
                    continue
                old_versions = plugin_entry.get("versions", {})
                new_versions = plugins[plugin_id]["versions"]
                for version, version_info in old_versions.items():
                    if version in new_versions:
                        # 版本仍存在：若 wasm 未变，保留旧的 released_at
                        old_sha = version_info.get("sha256")
                        new_sha = new_versions[version]["sha256"]
                        if old_sha == new_sha:
                            new_versions[version]["released_at"] = version_info.get(
                                "released_at", new_versions[version]["released_at"]
                            )
                    else:
                        # 版本已不存在于新扫描中：保留旧记录作为历史版本
                        new_versions[version] = version_info
                        print(f"  Preserved historical version: {plugin_id} @ {version}")
        except Exception as e:
            print(f"Warning: failed to merge old registry: {e}", file=sys.stderr)

    # 如果插件内容没有任何变化，保留旧的 updated_at，避免 CI 因时间戳差异而失败
    if old_registry is not None:
        def normalize_for_compare(obj):
            """用于比较两个 registry 是否内容相同（忽略 updated_at 和 released_at）"""
            copy = json.loads(json.dumps(obj))
            copy.pop("updated_at", None)
            for p in copy.get("plugins", {}).values():
                for v in p.get("versions", {}).values():
                    v.pop("released_at", None)
            return copy

        if normalize_for_compare(old_registry) == normalize_for_compare(registry):
            registry["updated_at"] = old_registry.get("updated_at", registry["updated_at"])
            print("  Plugin contents unchanged, preserving old updated_at")

    with open(REGISTRY_FILE, "w", encoding="utf-8") as f:
        json.dump(registry, f, indent=2, ensure_ascii=False)
        f.write("\n")

    print(f"Generated {REGISTRY_FILE} with {len(plugins)} plugin(s)")
    for pid, entry in plugins.items():
        print(f"  - {pid} @ {entry['latest_version']}")


if __name__ == "__main__":
    main()
