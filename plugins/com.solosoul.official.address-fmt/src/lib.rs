//! Address Formatter — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 将 Vault 中的所有地址按目标国家/地区规范格式化输出。

use solosoul_plugin_sdk::{get_field, log_error, log_info};

/// 地址组件
struct Address {
    street: String,
    city: String,
    state: String,
    postal_code: String,
    country: String,
    district: String,
}

/// 国家代码标准化（2字母或3字母转内部标识）
fn normalize_country(code: &str) -> &'static str {
    let upper = code.to_uppercase();
    match upper.as_str() {
        "CN" | "CHN" | "中国" | "CHINA" => "CN",
        "US" | "USA" | "美国" | "UNITED STATES" | "AMERICA" => "US",
        "GB" | "GBR" | "UK" | "英国" | "UNITED KINGDOM" | "BRITAIN" => "GB",
        "JP" | "JPN" | "日本" | "JAPAN" => "JP",
        "DE" | "DEU" | "德国" | "GERMANY" => "DE",
        "FR" | "FRA" | "法国" | "FRANCE" => "FR",
        "CA" | "CAN" | "加拿大" | "CANADA" => "CA",
        "AU" | "AUS" | "澳大利亚" | "AUSTRALIA" => "AU",
        "SG" | "SGP" | "新加坡" | "SINGAPORE" => "SG",
        "KR" | "KOR" | "韩国" | "SOUTH KOREA" => "KR",
        _ => "DEFAULT",
    }
}

/// 按国家格式化地址
fn format_address(addr: &Address) -> String {
    match normalize_country(&addr.country) {
        "CN" => format_china(addr),
        "US" => format_us(addr),
        "GB" => format_gb(addr),
        "JP" => format_jp(addr),
        "DE" => format_de(addr),
        "FR" => format_fr(addr),
        "CA" => format_ca(addr),
        "AU" => format_au(addr),
        "SG" => format_sg(addr),
        "KR" => format_kr(addr),
        _ => format_default(addr),
    }
}

/// 中国格式：省 + 市 + 区 + 街道 + 邮编
fn format_china(addr: &Address) -> String {
    let mut parts = Vec::new();
    if !addr.state.is_empty() { parts.push(addr.state.clone()); }
    if !addr.city.is_empty() { parts.push(addr.city.clone()); }
    if !addr.district.is_empty() { parts.push(addr.district.clone()); }
    if !addr.street.is_empty() { parts.push(addr.street.clone()); }

    let mut result = parts.join("");
    if !addr.postal_code.is_empty() {
        result.push_str(&format!(" ({})", addr.postal_code));
    }
    result
}

/// 美国格式：Street, City, State ZIP
fn format_us(addr: &Address) -> String {
    let mut result = addr.street.clone();
    if !addr.city.is_empty() {
        result.push_str(&format!(", {}", addr.city));
    }
    if !addr.state.is_empty() {
        result.push_str(&format!(", {}", addr.state));
    }
    if !addr.postal_code.is_empty() {
        result.push_str(&format!(" {}", addr.postal_code));
    }
    result
}

/// 英国格式：Street, City, County POSTCODE
fn format_gb(addr: &Address) -> String {
    let mut result = addr.street.clone();
    if !addr.city.is_empty() {
        result.push_str(&format!(", {}", addr.city));
    }
    if !addr.state.is_empty() {
        result.push_str(&format!(", {}", addr.state));
    }
    if !addr.postal_code.is_empty() {
        result.push_str(&format!(" {}", addr.postal_code));
    }
    result
}

/// 日本格式：〒ZIP + 都道府县 + 市区町村 + 番地
fn format_jp(addr: &Address) -> String {
    let mut result = String::new();
    if !addr.postal_code.is_empty() {
        result.push_str(&format!("〒{}", addr.postal_code));
    }
    if !addr.state.is_empty() {
        if !result.is_empty() { result.push(' '); }
        result.push_str(&addr.state);
    }
    if !addr.city.is_empty() {
        if !result.is_empty() { result.push(' '); }
        result.push_str(&addr.city);
    }
    if !addr.street.is_empty() {
        if !result.is_empty() { result.push(' '); }
        result.push_str(&addr.street);
    }
    result
}

/// 德国格式：Street, ZIP City
fn format_de(addr: &Address) -> String {
    let mut result = addr.street.clone();
    let city_part = if !addr.postal_code.is_empty() && !addr.city.is_empty() {
        format!(", {} {}", addr.postal_code, addr.city)
    } else if !addr.city.is_empty() {
        format!(", {}", addr.city)
    } else {
        String::new()
    };
    result.push_str(&city_part);
    if !addr.state.is_empty() {
        result.push_str(&format!(", {}", addr.state));
    }
    result
}

/// 法国格式：Street, ZIP City
fn format_fr(addr: &Address) -> String {
    format_de(addr) // 法国与德国格式相同
}

/// 加拿大格式：Street, City, Province POSTCODE
fn format_ca(addr: &Address) -> String {
    format_us(addr) // 加拿大与美国格式相同
}

/// 澳大利亚格式：Street, City, State POSTCODE
fn format_au(addr: &Address) -> String {
    format_us(addr) // 澳大利亚与美国格式相同
}

/// 新加坡格式：Street, City POSTCODE
fn format_sg(addr: &Address) -> String {
    let mut result = addr.street.clone();
    if !addr.city.is_empty() {
        result.push_str(&format!(", {}", addr.city));
    }
    if !addr.postal_code.is_empty() {
        result.push_str(&format!(" {}", addr.postal_code));
    }
    result
}

/// 韩国格式：省 + 市 + 区 + 街道 + 邮编
fn format_kr(addr: &Address) -> String {
    format_china(addr) // 韩国与中国格式逻辑相同
}

/// 默认格式（无国家匹配时）
fn format_default(addr: &Address) -> String {
    let mut result = addr.street.clone();
    if !addr.city.is_empty() {
        result.push_str(&format!(", {}", addr.city));
    }
    if !addr.state.is_empty() {
        result.push_str(&format!(", {}", addr.state));
    }
    if !addr.postal_code.is_empty() {
        result.push_str(&format!(" {}", addr.postal_code));
    }
    if !addr.country.is_empty() {
        result.push_str(&format!(", {}", addr.country));
    }
    result
}

/// 安全读取字段（带调试日志）
fn read_field(path: &str) -> String {
    match get_field(path) {
        Ok(value) => {
            let trimmed = value.trim().to_string();
            log_info(&format!("read_field('{}') OK value='{}'", path, &trimmed[..trimmed.len().min(80)]));
            trimmed
        }
        Err(e) => {
            log_error(&format!("read_field('{}') FAILED: {:?}", path, e));
            String::new()
        }
    }
}

/// 插件入口
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("Address Formatter 启动 — 格式化所有地址");

    let count_str = read_field("address.count");
    log_info(&format!("address.count raw='{}'", count_str));
    let count: usize = match count_str.parse() {
        Ok(n) => n,
        Err(e) => {
            log_error(&format!("无法解析地址数量: raw='{}' err={:?}", count_str, e));
            return -1;
        }
    };

    if count == 0 {
        log_error("未找到任何地址记录，请先添加至少一条地址记录");
        return -1;
    }

    log_info(&format!("发现 {} 条地址", count));
    let mut success_count = 0;

    for i in 0..count {
        log_info(&format!("--- 处理地址[{}] ---", i));
        let street = read_field(&format!("address[{}].street", i));
        let city = read_field(&format!("address[{}].city", i));
        let district = read_field(&format!("address[{}].district", i));
        let state = read_field(&format!("address[{}].state", i));
        let postal_code = read_field(&format!("address[{}].postalCode", i));
        let country = read_field(&format!("address[{}].country", i));

        log_info(&format!("地址[{}] street='{}' city='{}' state='{}' postal='{}' country='{}' district='{}'",
            i, street, city, state, postal_code, country, district));

        if street.is_empty() || city.is_empty() || country.is_empty() {
            log_error(&format!("地址[{}] 缺少必需字段: street='{}' city='{}' country='{}'", i, street, city, country));
            continue;
        }

        let addr = Address {
            street,
            city,
            state,
            postal_code,
            country: country.clone(),
            district,
        };

        let formatted = format_address(&addr);
        let country_label = normalize_country(&country);

        // label/title 用于标识地址含义（家/公司/老家等）
        let label = read_field(&format!("address[{}].label", i));
        let display_label = if label.is_empty() {
            read_field(&format!("address[{}].title", i))
        } else {
            label
        };

        log_info(&format!("地址[{}] 国家识别: {} → {}", i, country, country_label));
        if display_label.is_empty() {
            log_info(&format!("地址[{}] 格式化结果: {}", i, formatted));
        } else {
            log_info(&format!("地址[{}] 格式化结果: {} | {}", i, display_label, formatted));
        }
        success_count += 1;
    }

    log_info(&format!("Address Formatter 完成 — 成功格式化 {} / {} 条地址", success_count, count));

    if success_count == 0 {
        -1
    } else {
        0
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(street: &str, city: &str, state: &str, postal: &str, country: &str, district: &str) -> Address {
        Address {
            street: street.to_string(),
            city: city.to_string(),
            state: state.to_string(),
            postal_code: postal.to_string(),
            country: country.to_string(),
            district: district.to_string(),
        }
    }

    #[test]
    fn test_normalize_country() {
        assert_eq!(normalize_country("CN"), "CN");
        assert_eq!(normalize_country("chn"), "CN");
        assert_eq!(normalize_country("中国"), "CN");
        assert_eq!(normalize_country("US"), "US");
        assert_eq!(normalize_country("USA"), "US");
        assert_eq!(normalize_country("美国"), "US");
        assert_eq!(normalize_country("GB"), "GB");
        assert_eq!(normalize_country("UK"), "GB");
        assert_eq!(normalize_country("JP"), "JP");
        assert_eq!(normalize_country("日本"), "JP");
        assert_eq!(normalize_country("Unknown"), "DEFAULT");
    }

    #[test]
    fn test_format_china() {
        let a = addr("中关村大街1号", "北京市", "", "100080", "CN", "海淀区");
        assert_eq!(format_address(&a), "北京市海淀区中关村大街1号 (100080)");
    }

    #[test]
    fn test_format_us() {
        let a = addr("1600 Pennsylvania Avenue NW", "Washington", "DC", "20500", "US", "");
        assert_eq!(format_address(&a), "1600 Pennsylvania Avenue NW, Washington, DC 20500");
    }

    #[test]
    fn test_format_gb() {
        let a = addr("10 Downing Street", "London", "", "SW1A 2AA", "GB", "");
        assert_eq!(format_address(&a), "10 Downing Street, London SW1A 2AA");
    }

    #[test]
    fn test_format_jp() {
        let a = addr("1-1-2 丸の内", "千代田区", "東京都", "100-0001", "JP", "");
        assert_eq!(format_address(&a), "〒100-0001 東京都 千代田区 1-1-2 丸の内");
    }

    #[test]
    fn test_format_de() {
        let a = addr("Unter den Linden 1", "Berlin", "", "10117", "DE", "");
        assert_eq!(format_address(&a), "Unter den Linden 1, 10117 Berlin");
    }

    #[test]
    fn test_format_sg() {
        let a = addr("1 Raffles Place", "Singapore", "", "048616", "SG", "");
        assert_eq!(format_address(&a), "1 Raffles Place, Singapore 048616");
    }

    #[test]
    fn test_format_default() {
        let a = addr("Unknown St", "Mystery City", "", "", "XX", "");
        assert!(format_address(&a).contains("Unknown St"));
        assert!(format_address(&a).contains("XX"));
    }

    #[test]
    fn test_format_missing_optional() {
        let a = addr("Main St", "Springfield", "", "", "US", "");
        assert_eq!(format_address(&a), "Main St, Springfield");
    }
}
