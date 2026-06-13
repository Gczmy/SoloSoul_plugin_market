#!/usr/bin/env python3
"""为所有官方插件 manifest 添加 tier 与 category 字段。"""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PLUGINS_DIR = ROOT / "plugins"

TIER_MAP = {
    # P0: 纯本地、单功能、低风险
    "com.solosoul.official.address-fmt": ("p0", "formatter"),
    "com.solosoul.official.phone-fmt": ("p0", "formatter"),
    "com.solosoul.official.id-validator": ("p0", "validator"),
    "com.solosoul.official.totp-gen": ("p0", "generator"),
    "com.solosoul.official.namecard-gen": ("p0", "generator"),
    "com.solosoul.official.mrz-encoder": ("p0", "encoder"),
    "com.solosoul.official.expiry-guardian": ("p0", "reminder"),
    "com.solosoul.official.calendar-events": ("p0", "calendar"),
    "com.solosoul.official.contact-exporter": ("p0", "exporter"),
    "com.solosoul.official.packing-list": ("p0", "planner"),
    "com.solosoul.official.doc-checklist": ("p0", "planner"),
    "com.solosoul.official.travel-footprint": ("p0", "report"),
    "com.solosoul.official.identity-timeline": ("p0", "report"),
    # P1: 需要联网
    "com.solosoul.official.slotgo": ("p1", "network"),
    # P2: 多字段综合分析/生成
    "com.solosoul.official.data-completeness": ("p2", "report"),
    "com.solosoul.official.form-prefiller": ("p2", "report"),
    "com.solosoul.official.tax-profile": ("p2", "report"),
    "com.solosoul.official.resume-builder": ("p2", "generator"),
    "com.solosoul.official.emergency-card": ("p2", "generator"),
    "com.solosoul.official.digital-will": ("p2", "legal"),
}


def main():
    for manifest_path in sorted(PLUGINS_DIR.glob("*/manifest.json")):
        with manifest_path.open("r", encoding="utf-8") as f:
            data = json.load(f)
        plugin_id = data.get("plugin_id")
        tier, category = TIER_MAP.get(plugin_id, ("p3", "utility"))
        data["tier"] = tier
        data["category"] = category
        with manifest_path.open("w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
            f.write("\n")
        print(f"{plugin_id}: tier={tier}, category={category}")


if __name__ == "__main__":
    main()
