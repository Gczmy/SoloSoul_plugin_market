//! ID Validator — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 读取 Vault 中的证件号码，校验其格式与校验位合法性。

use solosoul_plugin_sdk::{get_field, log_error, log_info, send_result_json};

/// 中国居民身份证 18 位校验
///
/// 算法（GB 11643-1999）：
/// 1. 前 17 位分别乘以权重 [7,9,10,5,8,4,2,1,6,3,7,9,10,5,8,4,2]
/// 2. 求和对 11 取模
/// 3. 余数映射到校验码 [1,0,'X',9,8,7,6,5,4,3,2]
fn validate_cn_id(id: &str) -> Result<bool, &'static str> {
    if id.len() != 18 {
        return Err("中国身份证必须为 18 位");
    }

    // 检查前 17 位是否为数字
    let prefix = &id[..17];
    if !prefix.chars().all(|c| c.is_ascii_digit()) {
        return Err("前 17 位必须全为数字");
    }

    // 检查第 18 位：数字或 X/x
    let check_char = id.as_bytes()[17] as char;
    if !check_char.is_ascii_digit() && !matches!(check_char, 'X' | 'x') {
        return Err("第 18 位必须为数字或 X");
    }

    const WEIGHTS: [u32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
    const CHECK_CODES: [char; 11] = ['1', '0', 'X', '9', '8', '7', '6', '5', '4', '3', '2'];

    let mut sum: u32 = 0;
    for (i, ch) in prefix.chars().enumerate() {
        let digit = (ch as u8 - b'0') as u32;
        sum += digit * WEIGHTS[i];
    }

    let expected = CHECK_CODES[(sum % 11) as usize];
    let actual = check_char.to_ascii_uppercase();

    Ok(expected == actual)
}

/// 插件入口函数
///
/// 返回值：
/// - `0`: 校验通过或正常执行完毕
/// - `1`: 字段读取失败
/// - `2`: 校验不通过
/// 简单的 JSON 字符串转义
fn escape_json(s: &str) -> String {
    s.replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
}

#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("ID Validator 启动");

    // 1. 读取身份证字段
    let id_card = match get_field("idCard.number") {
        Ok(v) if !v.is_empty() => {
            log_info(&format!("读取到 idCard.number: {}", mask_id(&v)));
            v
        }
        Ok(_) => {
            log_info("idCard.number 为空，跳过身份证校验");
            String::new()
        }
        Err(e) => {
            log_error(&format!("读取 idCard.number 失败: {:?}", e));
            return 1;
        }
    };

    // 2. 校验身份证
    let mut id_valid = false;
    if !id_card.is_empty() {
        match validate_cn_id(&id_card) {
            Ok(true) => {
                log_info("✅ 身份证校验通过");
                id_valid = true;
            }
            Ok(false) => {
                log_error("❌ 身份证校验失败：校验位不匹配");
                return 2;
            }
            Err(msg) => {
                log_error(&format!("❌ 身份证格式错误: {}", msg));
                return 2;
            }
        }
    }

    // 3. 读取护照字段（可选）
    let mut passport = String::new();
    match get_field("passport.number") {
        Ok(v) if !v.is_empty() => {
            log_info(&format!("读取到 passport.number: {}", mask_id(&v)));
            passport = v;
            log_info("✅ 护照号码已记录（格式校验待扩展）");
        }
        Ok(_) => {
            log_info("passport.number 为空，跳过护照校验");
        }
        Err(_) => {
            log_info("passport.number 读取失败（可选字段，忽略）");
        }
    }

    log_info("ID Validator 执行完毕");

    // Phase 2: 结构化结果
    let mut pairs: Vec<(&str, String)> = Vec::new();
    if !id_card.is_empty() {
        pairs.push(("证件类型", "身份证".to_string()));
        pairs.push(("脱敏号码", mask_id(&id_card)));
        pairs.push(("校验结果", if id_valid { "✅ 通过".to_string() } else { "❌ 失败".to_string() }));
    }
    if !passport.is_empty() {
        pairs.push(("护照号码", mask_id(&passport)));
    }
    let pairs_json: Vec<String> = pairs.iter().map(|(k, v)| format!(r#"{{"key":"{}","value":"{}"}}"#, escape_json(k), escape_json(v))).collect();
    let result_json = format!(r#"{{"type":"key_value","title":"证件校验","pairs":[{}]}}"#, pairs_json.join(","));
    let _ = send_result_json(&result_json);

    0
}

/// 脱敏显示：保留前 3 位和后 2 位，中间用 * 替换
fn mask_id(id: &str) -> String {
    if id.len() <= 5 {
        return "***".to_string();
    }
    let prefix = &id[..3];
    let suffix = &id[id.len() - 2..];
    format!("{}****{}", prefix, suffix)
}

// ============================================================================
// 单元测试（wasm32-wasi 目标下需使用 wasm-bindgen-test，此处为文档测试）
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_cn_id() {
        // 这是一个结构有效的身份证号码（仅用于校验位算法测试）
        // 注意：此号码为随机生成，不对应任何真实个人
        let valid_id = "110101199001011237";
        assert!(validate_cn_id(valid_id).unwrap());
    }

    #[test]
    fn test_invalid_cn_id_wrong_check() {
        let invalid_id = "110101199001011230"; // 最后一位应为 X
        assert!(!validate_cn_id(invalid_id).unwrap());
    }

    #[test]
    fn test_invalid_cn_id_length() {
        assert!(validate_cn_id("123456").is_err());
        assert!(validate_cn_id("1234567890123456789").is_err());
    }

    #[test]
    fn test_invalid_cn_id_non_digit() {
        assert!(validate_cn_id("abcdefghijklmnopqr").is_err());
    }

    #[test]
    fn test_mask_id() {
        assert_eq!(mask_id("123456789012345678"), "123****78");
        assert_eq!(mask_id("abc"), "***");
    }
}
