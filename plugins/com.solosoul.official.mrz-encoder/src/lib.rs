//! MRZ Encoder — SoloSoul Official Plugin
//!
//! 纯本地插件，零网络依赖。
//! 将 Vault 中的护照信息编码为 ICAO Doc 9303 TD3 标准机读区格式。

use solosoul_plugin_sdk::{get_field, log_error, log_info};

/// MRZ 编码结果
struct MrzResult {
    line1: String,
    line2: String,
}

/// 字符到 MRZ 数值映射
/// 0-9 → 0-9, A-Z → 10-35, < → 0
fn char_value(c: char) -> u32 {
    match c {
        '0'..='9' => c as u32 - '0' as u32,
        'A'..='Z' => c as u32 - 'A' as u32 + 10,
        '<' => 0,
        _ => 0, // 非法字符视为填充符
    }
}

/// 计算 MRZ 校验位
/// 权重循环 [7, 3, 1]
fn compute_check_digit(data: &str) -> char {
    let weights = [7, 3, 1];
    let mut sum = 0u32;
    for (i, c) in data.chars().enumerate() {
        sum += char_value(c) * weights[i % 3];
    }
    let digit = (sum % 10) as u8;
    (b'0' + digit) as char
}

/// 格式化姓名字段
/// 姓氏 << 名字，空格替换为 <，不足用 < 填充，超出截断
fn format_name(surname: &str, given_names: &str, max_len: usize) -> String {
    let mut result = String::with_capacity(max_len);

    // 姓氏：空格/逗号替换为 <
    for c in surname.to_uppercase().chars() {
        if c == ' ' || c == ',' || c == '-' {
            result.push('<');
        } else if c.is_ascii_alphanumeric() {
            result.push(c);
        }
    }

    result.push('<');
    result.push('<');

    // 名字
    for c in given_names.to_uppercase().chars() {
        if c == ' ' || c == ',' || c == '-' {
            result.push('<');
        } else if c.is_ascii_alphanumeric() {
            result.push(c);
        }
    }

    // 截断或填充
    if result.len() > max_len {
        result.truncate(max_len);
    } else {
        while result.len() < max_len {
            result.push('<');
        }
    }

    result
}

/// 将日期从 YYYY-MM-DD 格式转换为 YYMMDD
fn to_mrz_date(date_str: &str) -> Result<String, &'static str> {
    let s = date_str.trim();
    if s.len() == 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
        Ok(format!("{}{}{}", &s[2..4], &s[5..7], &s[8..10]))
    } else if s.len() == 6 && s.chars().all(|c| c.is_ascii_digit()) {
        // 已经是 YYMMDD 格式
        Ok(s.to_string())
    } else {
        Err("日期格式必须为 YYYY-MM-DD 或 YYMMDD")
    }
}

/// 格式化护照号码（9位，不足用 < 填充）
fn format_doc_number(doc_no: &str) -> String {
    let upper = doc_no.to_uppercase();
    let mut result = upper.chars().take(9).collect::<String>();
    while result.len() < 9 {
        result.push('<');
    }
    result
}

/// 格式化个人号码（14位，不足用 < 填充）
fn format_personal_number(pn: &str) -> String {
    let upper = pn.to_uppercase();
    let mut result = upper.chars().take(14).collect::<String>();
    while result.len() < 14 {
        result.push('<');
    }
    result
}

/// 编码 TD3（护照）MRZ
fn encode_td3(
    issuing_country: &str,
    surname: &str,
    given_names: &str,
    doc_number: &str,
    nationality: &str,
    dob: &str,
    sex: &str,
    expiry: &str,
    personal_number: &str,
) -> Result<MrzResult, &'static str> {
    // Line 1: P< + issuing_country(3) + name(39) = 44
    let country = issuing_country.to_uppercase();
    if country.len() != 3 {
        return Err("签发国代码必须为 3 位字母（如 CHN）");
    }

    let name_field = format_name(surname, given_names, 39);
    let line1 = format!("P<{}{}", country, name_field);
    debug_assert_eq!(line1.len(), 44, "Line 1 长度必须为 44");

    // Line 2
    let doc_num = format_doc_number(doc_number);
    let doc_check = compute_check_digit(&doc_num);

    let nat = nationality.to_uppercase();
    if nat.len() != 3 {
        return Err("国籍代码必须为 3 位字母（如 CHN）");
    }

    let dob_mrz = to_mrz_date(dob)?;
    let dob_check = compute_check_digit(&dob_mrz);

    let sex_mrz = match sex.to_uppercase().as_str() {
        "M" | "MALE" => "M",
        "F" | "FEMALE" => "F",
        _ => "<",
    };

    let exp_mrz = to_mrz_date(expiry)?;
    let exp_check = compute_check_digit(&exp_mrz);

    let pers_num = format_personal_number(personal_number);
    let pers_check = compute_check_digit(&pers_num);

    // Composite check digit over:
    // doc_num(9) + doc_check(1) + dob(6) + dob_check(1) + expiry(6) + exp_check(1) + pers_num(14) + pers_check(1) = 39 chars
    let composite_data = format!(
        "{}{}{}{}{}{}{}{}",
        doc_num, doc_check, dob_mrz, dob_check, exp_mrz, exp_check, pers_num, pers_check
    );
    let composite_check = compute_check_digit(&composite_data);

    let line2 = format!(
        "{}{}{}{}{}{}{}{}{}{}{}",
        doc_num, doc_check, nat, dob_mrz, dob_check, sex_mrz, exp_mrz, exp_check, pers_num, pers_check, composite_check
    );

    // 长度校验
    if line2.len() != 44 {
        return Err("Line 2 长度异常");
    }

    Ok(MrzResult { line1, line2 })
}

/// 脱敏显示 MRZ（隐藏敏感字符）
fn mask_mrz(result: &MrzResult) -> String {
    format!(
        "{}\n{}{}",
        &result.line1[..6],
        &result.line2[..9],
        "*".repeat(result.line2.len() - 9)
    )
}

/// 插件入口
#[no_mangle]
pub extern "C" fn run() -> i32 {
    log_info("MRZ Encoder 启动 — 编码护照 MRZ");

    // 读取所有必需字段
    let fields = [
        ("passport.number", "护照号码"),
        ("passport.nationality", "国籍"),
        ("passport.surname", "姓氏"),
        ("passport.givenNames", "名字"),
        ("passport.dateOfBirth", "出生日期"),
        ("passport.sex", "性别"),
        ("passport.expiryDate", "有效期"),
    ];

    let mut values: Vec<Option<String>> = Vec::with_capacity(fields.len());
    let mut missing = Vec::new();

    for (path, name) in &fields {
        match get_field(path) {
            Ok(v) if !v.trim().is_empty() => values.push(Some(v)),
            _ => {
                values.push(None);
                missing.push(*name);
            }
        }
    }

    if !missing.is_empty() {
        log_error(&format!("缺少必需字段: {}", missing.join(", ")));
        return -1;
    }

    // 提取值
    let doc_number = values[0].as_ref().unwrap();
    let nationality = values[1].as_ref().unwrap();
    let surname = values[2].as_ref().unwrap();
    let given_names = values[3].as_ref().unwrap();
    let dob = values[4].as_ref().unwrap();
    let sex = values[5].as_ref().unwrap();
    let expiry = values[6].as_ref().unwrap();

    // 签发国默认使用国籍（大多数情况）
    let issuing_country = nationality.trim();

    // 可选字段：个人号码
    let personal_number = get_field("passport.personalNumber")
        .unwrap_or_default();

    match encode_td3(
        issuing_country,
        surname,
        given_names,
        doc_number,
        nationality,
        dob,
        sex,
        expiry,
        &personal_number,
    ) {
        Ok(result) => {
            log_info("MRZ 编码成功:");
            log_info(&result.line1);
            log_info(&result.line2);
            log_info(&format!("脱敏预览:\n{}", mask_mrz(&result)));
            0
        }
        Err(e) => {
            log_error(&format!("MRZ 编码失败: {}", e));
            -2
        }
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_value() {
        assert_eq!(char_value('0'), 0);
        assert_eq!(char_value('9'), 9);
        assert_eq!(char_value('A'), 10);
        assert_eq!(char_value('Z'), 35);
        assert_eq!(char_value('<'), 0);
    }

    #[test]
    fn test_compute_check_digit() {
        // ICAO 9303 校验位算法验证
        // "123456789": 1*7+2*3+3*1+4*7+5*3+6*1+7*7+8*3+9*1 = 147 → 7
        assert_eq!(compute_check_digit("123456789"), '7');
        // "ABCD": 10*7+11*3+12*1+13*7 = 206 → 6
        assert_eq!(compute_check_digit("ABCD"), '6');
        // "<<": 0*7+0*3 = 0 → 0
        assert_eq!(compute_check_digit("<<"), '0');
        // "": sum=0 → 0
        assert_eq!(compute_check_digit(""), '0');
    }

    #[test]
    fn test_format_name() {
        assert_eq!(
            format_name("ZHANG", "SAN", 39),
            "ZHANG<<SAN<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"
        );
        assert_eq!(
            format_name("VAN DER", "JOHN PAUL", 39),
            "VAN<DER<<JOHN<PAUL<<<<<<<<<<<<<<<<<<<<<"
        );
    }

    #[test]
    fn test_to_mrz_date() {
        assert_eq!(to_mrz_date("1990-05-23").unwrap(), "900523");
        assert_eq!(to_mrz_date("2000-01-01").unwrap(), "000101");
        assert_eq!(to_mrz_date("900523").unwrap(), "900523");
        assert!(to_mrz_date("invalid").is_err());
    }

    #[test]
    fn test_format_doc_number() {
        assert_eq!(format_doc_number("E12345678"), "E12345678");
        assert_eq!(format_doc_number("ABC"), "ABC<<<<<<");
    }

    #[test]
    fn test_encode_td3_basic() {
        let result = encode_td3(
            "CHN", "ZHANG", "SAN",
            "E12345678", "CHN", "1990-05-23", "M", "2025-12-31", ""
        ).unwrap();

        assert_eq!(result.line1.len(), 44);
        assert!(result.line1.starts_with("P<CHNZHANG<<SAN"));

        assert_eq!(result.line2.len(), 44);
        assert!(result.line2.starts_with("E12345678"));
    }

    #[test]
    fn test_encode_td3_with_personal_number() {
        let result = encode_td3(
            "CHN", "LI", "SI",
            "G87654321", "CHN", "1985-10-10", "F", "2030-06-15", "ID123456"
        ).unwrap();

        assert_eq!(result.line1.len(), 44);
        assert_eq!(result.line2.len(), 44);
    }

    #[test]
    fn test_mask_mrz() {
        let result = MrzResult {
            line1: "P<CHNZHANG<<SAN<<<<<<<<<<<<<<<<<<<<<<<<<".to_string(),
            line2: "E123456780CHN9005237M2512317<<<<<<<<<<04".to_string(),
        };
        let masked = mask_mrz(&result);
        assert!(masked.contains("P<CHNZ"));
        assert!(masked.contains("E12345678"));
        assert!(masked.contains("*********"));
    }
}
