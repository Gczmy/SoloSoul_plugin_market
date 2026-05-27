//! Expiry Guardian — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 扫描 Vault 中所有证件的有效期，计算剩余天数并按紧急程度分级预警。

use solosoul_plugin_sdk::{get_field, get_timestamp, log_error, log_info, send_result_json};

/// 证件条目
struct Document {
    name: &'static str,
    field_path: &'static str,
    value: String,
}

/// 紧急程度分级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Urgency {
    Safe,      // > 180 天
    Notice,    // 90-180 天
    Warning,   // 60-90 天
    Critical,  // 30-60 天
    Expired,   // <= 0 天（已过期）
}

impl Urgency {
    fn label(&self) -> &'static str {
        match self {
            Urgency::Safe => "✅ 安全",
            Urgency::Notice => "ℹ️ 注意",
            Urgency::Warning => "⚠️ 警告",
            Urgency::Critical => "🔴 紧急",
            Urgency::Expired => "❌ 已过期",
        }
    }

    fn from_days(days: i64) -> Self {
        match days {
            d if d <= 0 => Urgency::Expired,
            d if d <= 30 => Urgency::Critical,
            d if d <= 60 => Urgency::Warning,
            d if d <= 90 => Urgency::Notice,
            _ => Urgency::Safe,
        }
    }
}

/// 解析日期字符串，支持两种格式：
/// - ISO: "2025-12-31"
/// - MRZ/紧凑: "251231" (解释为 2025-12-31)
fn parse_date(date_str: &str) -> Option<(i32, u32, u32)> {
    let s = date_str.trim();

    if s.len() == 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
        // ISO 格式: YYYY-MM-DD
        let year = s[..4].parse().ok()?;
        let month = s[5..7].parse().ok()?;
        let day = s[8..10].parse().ok()?;
        return Some((year, month, day));
    }

    if s.len() == 6 && s.chars().all(|c| c.is_ascii_digit()) {
        // MRZ 格式: YYMMDD
        let yy: i32 = s[..2].parse().ok()?;
        let month: u32 = s[2..4].parse().ok()?;
        let day: u32 = s[4..6].parse().ok()?;
        // 世纪推断：>=50 为 19YY，<50 为 20YY
        let year = if yy >= 50 { 1900 + yy } else { 2000 + yy };
        return Some((year, month, day));
    }

    None
}

/// 计算从当前日期到目标日期的剩余天数
/// 使用简化算法：计算两个日期的序数差
fn days_until(year: i32, month: u32, day: u32, current_year: i32, current_month: u32, current_day: u32) -> i64 {
    fn ordinal(y: i32, m: u32, d: u32) -> i64 {
        // 简化儒略日数计算 (Fliegel-Van Flandern 算法)
        let a = (14 - m as i32) / 12;
        let y_adjusted = y + 4800 - a;
        let m_adjusted = m as i32 + 12 * a - 3;
        let jd = d as i64
            + ((153 * m_adjusted + 2) / 5) as i64
            + 365 * y_adjusted as i64
            + y_adjusted as i64 / 4
            - y_adjusted as i64 / 100
            + y_adjusted as i64 / 400
            - 32045;
        jd
    }

    ordinal(year, month, day) - ordinal(current_year, current_month, current_day)
}

/// 从 Unix 时间戳（毫秒）解析出年月日
fn timestamp_to_ymd(ts_ms: i64) -> (i32, u32, u32) {
    // 使用简化算法将 Unix 天数转换为年月日
    let days_since_epoch = ts_ms / 86400000;
    // 1970-01-01 的简化儒略日
    let jd_1970 = 2440588;
    let jd = jd_1970 + days_since_epoch;

    let l = jd + 68569;
    let n = (4 * l) / 146097;
    let l = l - (146097 * n + 3) / 4;
    let i = (4000 * (l + 1)) / 1461001;
    let l = l - (1461 * i) / 4 + 31;
    let j = (80 * l) / 2447;
    let d = l - (2447 * j) / 80;
    let l = j / 11;
    let m = j + 2 - 12 * l;
    let y = 100 * (n - 49) + i + l;

    (y as i32, m as u32, d as u32)
}

/// 检查单个证件
fn check_document(doc: &Document, current_year: i32, current_month: u32, current_day: u32) {
    if doc.value.is_empty() {
        log_info(&format!("{}: 未填写", doc.name));
        return;
    }

    match parse_date(&doc.value) {
        Some((year, month, day)) => {
            let days = days_until(year, month, day, current_year, current_month, current_day);
            let urgency = Urgency::from_days(days);

            log_info(&format!(
                "{}: {} ({} 天后到期) — {}",
                doc.name,
                doc.value,
                days,
                urgency.label()
            ));
        }
        None => {
            log_error(&format!(
                "{}: 日期格式无法解析 '{}' (期望 YYYY-MM-DD 或 YYMMDD)",
                doc.name, doc.value
            ));
        }
    }
}

/// 简单的 JSON 字符串转义
fn escape_json(s: &str) -> String {
    s.replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
}

/// 插件入口函数
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("Expiry Guardian 启动 — 扫描证件有效期");

    // 获取当前日期
    let now_ms = get_timestamp();
    let (current_year, current_month, current_day) = timestamp_to_ymd(now_ms);
    log_info(&format!(
        "当前日期: {}-{:02}-{:02}",
        current_year, current_month, current_day
    ));

    // 定义要扫描的证件列表
    let docs_to_check = [
        Document {
            name: "护照",
            field_path: "passport.expiryDate",
            value: String::new(),
        },
        Document {
            name: "身份证",
            field_path: "idCard.expiryDate",
            value: String::new(),
        },
        Document {
            name: "签证",
            field_path: "visa.expiryDate",
            value: String::new(),
        },
        Document {
            name: "信用卡",
            field_path: "card.expiryDate",
            value: String::new(),
        },
    ];

    let mut checked_count = 0;
    let mut result_pairs: Vec<(String, String)> = Vec::new();

    for doc in &docs_to_check {
        match get_field(doc.field_path) {
            Ok(value) => {
                let doc_with_value = Document {
                    name: doc.name,
                    field_path: doc.field_path,
                    value: value.clone(),
                };
                check_document(&doc_with_value, current_year, current_month, current_day);
                checked_count += 1;
                result_pairs.push((doc.name.to_string(), value));
            }
            Err(e) => {
                // 可选字段读取失败不阻断
                log_info(&format!("{}: 无法读取 ({:?})", doc.name, e));
            }
        }
    }

    log_info(&format!(
        "Expiry Guardian 扫描完毕 — 检查了 {} 个证件",
        checked_count
    ));

    // Phase 2: 结构化结果
    let pairs_json: Vec<String> = result_pairs.iter().map(|(k, v)| {
        format!(r#"{{"key":"{}","value":"{}"}}"#, escape_json(k), escape_json(v))
    }).collect();
    let result_json = format!(r#"{{"type":"key_value","title":"证件有效期","pairs":[{}]}}"#, pairs_json.join(","));
    let _ = send_result_json(&result_json);

    0
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date_iso() {
        assert_eq!(parse_date("2025-12-31"), Some((2025, 12, 31)));
        assert_eq!(parse_date("2020-01-01"), Some((2020, 1, 1)));
    }

    #[test]
    fn test_parse_date_mrz() {
        assert_eq!(parse_date("251231"), Some((2025, 12, 31)));
        assert_eq!(parse_date("001231"), Some((2000, 12, 31)));
        assert_eq!(parse_date("991231"), Some((1999, 12, 31)));
        assert_eq!(parse_date("500101"), Some((1950, 1, 1)));
    }

    #[test]
    fn test_parse_date_invalid() {
        assert_eq!(parse_date(""), None);
        assert_eq!(parse_date("not-a-date"), None);
        assert_eq!(parse_date("2025/12/31"), None);
    }

    #[test]
    fn test_days_until() {
        // 2025-01-01 到 2025-01-10 = 9 天
        assert_eq!(days_until(2025, 1, 10, 2025, 1, 1), 9);
        // 2025-01-01 到 2024-12-31 = -1 天（已过期）
        assert_eq!(days_until(2024, 12, 31, 2025, 1, 1), -1);
    }

    #[test]
    fn test_urgency_from_days() {
        assert_eq!(Urgency::from_days(-5), Urgency::Expired);
        assert_eq!(Urgency::from_days(0), Urgency::Expired);
        assert_eq!(Urgency::from_days(15), Urgency::Critical);
        assert_eq!(Urgency::from_days(45), Urgency::Warning);
        assert_eq!(Urgency::from_days(75), Urgency::Notice);
        assert_eq!(Urgency::from_days(200), Urgency::Safe);
    }

    #[test]
    fn test_timestamp_to_ymd_basic() {
        // 2024-01-01 00:00:00 UTC = 1704067200000 ms
        let (y, m, d) = timestamp_to_ymd(1704067200000);
        assert_eq!((y, m, d), (2024, 1, 1));
    }
}
