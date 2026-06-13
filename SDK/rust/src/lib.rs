//! SoloSoul Plugin SDK — Rust
//!
//! 为插件开发者提供类型安全的 Host Functions 绑定。
//! 插件编译目标：`wasm32-wasi`

use std::mem::MaybeUninit;

// ============================================================================
// Host Functions ABI (由 SoloSoul 主软件 Rust Host 侧实现)
// ============================================================================

extern "C" {
    /// 请求用户字段数据
    ///
    /// # 参数
    /// - `field_id_ptr`: 字段路径 UTF-8 字节指针（如 "identity.full_name"）
    /// - `field_id_len`: 字段路径长度
    /// - `out_ptr`: 输出缓冲区指针
    /// - `out_cap`: 输出缓冲区容量
    ///
    /// # 返回值
    /// - `0`: 成功
    /// - `-1`: 权限不足（字段不在 manifest 声明范围内）
    /// - `-2`: 用户拒绝
    /// - `-3`: TTL 过期或 Session 被撤销
    /// - `-4`: 缓冲区不足（out_cap 太小）
    /// - `-5`: 字段路径非法
    /// - `-7`: Vault 已锁定（用户未登录）
    /// - `-8`: 频率超限（同一字段 > 10 次/分钟）
    fn solosoul_request_field(
        field_id_ptr: *const u8,
        field_id_len: usize,
        out_ptr: *mut u8,
        out_cap: usize,
    ) -> i32;

    /// 代理 HTTP POST 请求（域名白名单限制）
    ///
    /// # 参数
    /// - `url_ptr`: URL UTF-8 字节指针
    /// - `url_len`: URL 长度
    /// - `body_ptr`: 请求体 UTF-8 字节指针
    /// - `body_len`: 请求体长度
    /// - `out_ptr`: 输出缓冲区指针
    /// - `out_cap`: 输出缓冲区容量
    ///
    /// # 返回值
    /// - `0`: 成功
    /// - `-6`: 网络超时（> 30 秒）
    /// - `-10`: 域名未授权（URL 不在 manifest 白名单）
    fn solosoul_post_data(
        url_ptr: *const u8,
        url_len: usize,
        body_ptr: *const u8,
        body_len: usize,
        out_ptr: *mut u8,
        out_cap: usize,
    ) -> i32;

    /// 发起异步 HTTP 请求
    ///
    /// # 参数
    /// - `method_ptr`: HTTP 方法 UTF-8 字节指针（"GET"/"POST"/"PUT"/"PATCH"/"DELETE"）
    /// - `method_len`: 方法长度
    /// - `url_ptr`: URL UTF-8 字节指针
    /// - `url_len`: URL 长度
    /// - `body_ptr`: 请求体 UTF-8 字节指针（可为空）
    /// - `body_len`: 请求体长度
    /// - `out_handle_ptr`: 输出句柄（u32 little-endian）指针
    ///
    /// # 返回值
    /// - `0`: 成功
    /// - `-6`: 网络超时
    /// - `-8`: 频率超限
    /// - `-10`: 域名未授权
    /// - `-11`: 非法参数
    fn solosoul_http_request(
        method_ptr: *const u8,
        method_len: usize,
        url_ptr: *const u8,
        url_len: usize,
        body_ptr: *const u8,
        body_len: usize,
        out_handle_ptr: *mut u8,
    ) -> i32;

    /// 轮询异步 HTTP 请求状态
    ///
    /// # 参数
    /// - `handle`: 请求句柄
    /// - `out_status_ptr`: 输出 HTTP 状态码（u16 little-endian）指针
    /// - `out_len_ptr`: 输出响应体长度（u32 little-endian）指针
    ///
    /// # 返回值
    /// - `0`: 已完成
    /// - `1`: 进行中
    /// - 负数: 错误码
    fn solosoul_http_poll(handle: u32, out_status_ptr: *mut u8, out_len_ptr: *mut u8) -> i32;

    /// 读取异步 HTTP 响应体
    ///
    /// # 参数
    /// - `handle`: 请求句柄
    /// - `out_ptr`: 输出缓冲区指针
    /// - `out_cap`: 输出缓冲区容量
    /// - `written_ptr`: 实际写入长度（u32 little-endian）指针
    ///
    /// # 返回值
    /// - `0`: 成功
    /// - `-4`: 缓冲区不足
    /// - 负数: 错误码
    fn solosoul_http_read(
        handle: u32,
        out_ptr: *mut u8,
        out_cap: usize,
        written_ptr: *mut u8,
    ) -> i32;

    /// 关闭异步 HTTP 请求句柄
    ///
    /// # 参数
    /// - `handle`: 请求句柄
    ///
    /// # 返回值
    /// - `0`: 成功
    /// - `-11`: 非法句柄
    fn solosoul_http_close(handle: u32) -> i32;

    /// 同步睡眠（毫秒）
    fn solosoul_sleep(ms: i64) -> i32;

    /// 写审计日志
    ///
    /// # 参数
    /// - `level_ptr`: 日志级别 UTF-8 字节指针（"debug" / "info" / "warn" / "error"）
    /// - `level_len`: 日志级别长度
    /// - `msg_ptr`: 消息 UTF-8 字节指针
    /// - `msg_len`: 消息长度
    fn solosoul_log(level_ptr: *const u8, level_len: usize, msg_ptr: *const u8, msg_len: usize);

    /// 获取 Unix 时间戳（毫秒）
    fn solosoul_get_timestamp() -> i64;

    /// 获取用户数据结构树（Phase 3）
    ///
    /// # 参数
    /// - `out_ptr`: 输出缓冲区指针
    /// - `out_cap`: 输出缓冲区容量
    ///
    /// # 返回值
    /// - `0`: 成功
    /// - `-1`: 错误（Vault 锁定、无数据等）
    /// - `-4`: 缓冲区不足（out_cap 太小）
    fn solosoul_get_data_structure_tree(out_ptr: *mut u8, out_cap: usize) -> i32;

    /// 发送结构化最终结果（Phase 2）
    ///
    /// # 参数
    /// - `data_ptr`: JSON 数据 UTF-8 字节指针
    /// - `data_len`: JSON 数据长度
    ///
    /// # 返回值
    /// - `0`: 成功
    /// - `-1`: 大小超限（> 64KB）
    /// - `-2`: 编码非法（非 UTF-8）
    /// - `-3`: 嵌套深度超限（> 10）
    /// - `-4`: 非法 type
    /// - `-5`: 缺少 type 字段
    /// - `-6`: 非法 JSON
    fn solosoul_result(data_ptr: *const u8, data_len: usize) -> i32;

    /// 显示通用对话框（Phase 4）
    ///
    /// # 参数
    /// - `config_ptr`: 对话框配置 JSON UTF-8 字节指针
    /// - `config_len`: 配置 JSON 长度
    /// - `out_ptr`: 输出缓冲区指针
    /// - `out_cap`: 输出缓冲区容量
    ///
    /// # 返回值
    /// - `0`: 成功
    /// - `-1`: 通道关闭
    /// - `-2`: 用户取消
    /// - `-3`: 超时
    /// - `-4`: 缓冲区不足
    /// - `-5`: 无权限
    fn solosoul_show_dialog(
        config_ptr: *const u8,
        config_len: usize,
        out_ptr: *mut u8,
        out_cap: usize,
    ) -> i32;
}

// ============================================================================
// 安全的 Rust 封装
// ============================================================================

/// SDK 错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginError {
    /// 权限不足
    PermissionDenied = -1,
    /// 用户拒绝
    UserDenied = -2,
    /// TTL 过期或超时
    TtlExpired = -3,
    /// 缓冲区不足
    BufferTooSmall = -4,
    /// 字段路径非法
    InvalidField = -5,
    /// 网络超时
    NetworkTimeout = -6,
    /// Vault 已锁定
    VaultLocked = -7,
    /// 频率超限
    RateLimited = -8,
    /// 域名未授权
    DomainNotAllowed = -10,
    /// 对话框失败
    DialogFailed = -11,
    /// 未知错误
    Unknown = -99,
}

impl PluginError {
    fn from_code(code: i32) -> Self {
        match code {
            -1 => PluginError::PermissionDenied,
            -2 => PluginError::UserDenied,
            -3 => PluginError::TtlExpired,
            -4 => PluginError::BufferTooSmall,
            -5 => PluginError::InvalidField,
            -6 => PluginError::NetworkTimeout,
            -7 => PluginError::VaultLocked,
            -8 => PluginError::RateLimited,
            -10 => PluginError::DomainNotAllowed,
            _ => PluginError::Unknown,
        }
    }
}

/// 请求用户字段数据
///
/// # 示例
/// ```ignore
/// let name = get_field("identity.full_name").expect("获取姓名失败");
/// ```
pub fn get_field(field_id: &str) -> Result<String, PluginError> {
    const INITIAL_CAP: usize = 4096;
    let mut buf: [MaybeUninit<u8>; INITIAL_CAP] = [MaybeUninit::uninit(); INITIAL_CAP];

    let code = unsafe {
        solosoul_request_field(
            field_id.as_ptr(),
            field_id.len(),
            buf.as_mut_ptr() as *mut u8,
            INITIAL_CAP,
        )
    };

    if code != 0 {
        return Err(PluginError::from_code(code));
    }

    // 安全：Host 已写入有效 UTF-8 数据
    let len = unsafe {
        let mut end = INITIAL_CAP;
        for (i, b) in buf.iter().enumerate() {
            if unsafe { b.assume_init() } == 0 {
                end = i;
                break;
            }
        }
        end
    };

    let bytes: Vec<u8> = buf[..len]
        .iter()
        .map(|b| unsafe { b.assume_init() })
        .collect();

    // Host 保证返回 UTF-8，如果解析失败视为 Unknown
    String::from_utf8(bytes).map_err(|_| PluginError::Unknown)
}

/// 代理 HTTP POST 请求（JSON）
///
/// # 示例
/// ```ignore
/// let resp = post_json("https://api.example.com/submit", r#"{"name":"Alice"}"#)
///     .expect("网络请求失败");
/// ```
pub fn post_json(url: &str, json_body: &str) -> Result<String, PluginError> {
    const INITIAL_CAP: usize = 65536;
    let mut buf: [MaybeUninit<u8>; INITIAL_CAP] = [MaybeUninit::uninit(); INITIAL_CAP];

    let code = unsafe {
        solosoul_post_data(
            url.as_ptr(),
            url.len(),
            json_body.as_ptr(),
            json_body.len(),
            buf.as_mut_ptr() as *mut u8,
            INITIAL_CAP,
        )
    };

    if code != 0 {
        return Err(PluginError::from_code(code));
    }

    // 查找 null terminator 或完整缓冲区
    let len = unsafe {
        let mut end = INITIAL_CAP;
        for (i, b) in buf.iter().enumerate() {
            if unsafe { b.assume_init() } == 0 {
                end = i;
                break;
            }
        }
        end
    };

    let bytes: Vec<u8> = buf[..len]
        .iter()
        .map(|b| unsafe { b.assume_init() })
        .collect();

    String::from_utf8(bytes).map_err(|_| PluginError::Unknown)
}

/// 异步 HTTP 请求轮询状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpPollState {
    /// 请求仍在进行中
    Pending,
    /// 请求已完成
    Completed {
        /// HTTP 状态码
        status: u16,
        /// 响应体字节长度
        body_len: usize,
    },
}

/// 异步 HTTP 请求句柄
///
/// # 示例
/// ```ignore
/// let req = HttpRequest::new("POST", "https://api.example.com/submit", r#"{"x":1}"#)?;
/// loop {
///     match req.poll()? {
///         HttpPollState::Pending => sleep(10),
///         HttpPollState::Completed { body_len, .. } => {
///             let mut buf = vec![MaybeUninit::<u8>::uninit(); body_len + 1];
///             let resp = req.read(&mut buf)?;
///             break;
///         }
///     }
/// }
/// ```
pub struct HttpRequest {
    handle: u32,
}

impl HttpRequest {
    /// 发起异步 HTTP 请求
    pub fn new(method: &str, url: &str, body: &str) -> Result<Self, PluginError> {
        let mut handle_buf: [MaybeUninit<u8>; 4] = [MaybeUninit::uninit(); 4];

        let code = unsafe {
            solosoul_http_request(
                method.as_ptr(),
                method.len(),
                url.as_ptr(),
                url.len(),
                body.as_ptr(),
                body.len(),
                handle_buf.as_mut_ptr() as *mut u8,
            )
        };

        if code != 0 {
            return Err(PluginError::from_code(code));
        }

        let handle = read_u32(&handle_buf);
        Ok(Self { handle })
    }

    /// 轮询请求状态
    pub fn poll(&self) -> Result<HttpPollState, PluginError> {
        let mut status_buf: [MaybeUninit<u8>; 2] = [MaybeUninit::uninit(); 2];
        let mut len_buf: [MaybeUninit<u8>; 4] = [MaybeUninit::uninit(); 4];

        let code = unsafe {
            solosoul_http_poll(
                self.handle,
                status_buf.as_mut_ptr() as *mut u8,
                len_buf.as_mut_ptr() as *mut u8,
            )
        };

        match code {
            0 => Ok(HttpPollState::Completed {
                status: read_u16(&status_buf),
                body_len: read_u32(&len_buf) as usize,
            }),
            1 => Ok(HttpPollState::Pending),
            _ => Err(PluginError::from_code(code)),
        }
    }

    /// 读取已完成请求的响应体
    ///
    /// `buf` 容量应至少为 `body_len + 1`（包含 Host 写入的结尾 `\0`）。
    pub fn read(&self, buf: &mut [MaybeUninit<u8>]) -> Result<String, PluginError> {
        let mut written_buf: [MaybeUninit<u8>; 4] = [MaybeUninit::uninit(); 4];

        let code = unsafe {
            solosoul_http_read(
                self.handle,
                buf.as_mut_ptr() as *mut u8,
                buf.len(),
                written_buf.as_mut_ptr() as *mut u8,
            )
        };

        if code != 0 {
            return Err(PluginError::from_code(code));
        }

        let len = read_u32(&written_buf) as usize;
        let bytes: Vec<u8> = buf[..len]
            .iter()
            .map(|b| unsafe { b.assume_init() })
            .collect();

        String::from_utf8(bytes).map_err(|_| PluginError::Unknown)
    }

    /// 关闭请求句柄，释放资源
    pub fn close(self) {
        unsafe {
            let _ = solosoul_http_close(self.handle);
        }
    }
}

/// 同步睡眠（毫秒）
///
/// # 示例
/// ```ignore
/// sleep(10);
/// ```
pub fn sleep(ms: i64) {
    unsafe {
        let _ = solosoul_sleep(ms);
    }
}

/// 异步 POST JSON（内部轮询，直到请求完成）
///
/// 如果需要在请求过程中执行其他逻辑，请直接使用 [`HttpRequest`]。
pub fn post_json_async(url: &str, json_body: &str) -> Result<String, PluginError> {
    let req = HttpRequest::new("POST", url, json_body)?;
    loop {
        match req.poll()? {
            HttpPollState::Pending => sleep(10),
            HttpPollState::Completed { body_len, .. } => {
                let mut buf = vec![MaybeUninit::<u8>::uninit(); body_len + 1];
                return req.read(&mut buf);
            }
        }
    }
}

/// 写审计日志
///
/// # 示例
/// ```ignore
/// log_info("开始预约流程");
/// log_error("预约失败：时间冲突");
/// ```
pub fn log(level: &str, message: &str) {
    unsafe {
        solosoul_log(level.as_ptr(), level.len(), message.as_ptr(), message.len());
    }
}

/// info 级别日志
pub fn log_info(message: &str) {
    log("info", message);
}

/// error 级别日志
pub fn log_error(message: &str) {
    log("error", message);
}

/// debug 级别日志
pub fn log_debug(message: &str) {
    log("debug", message);
}

/// 获取 Unix 时间戳（毫秒）
///
/// # 示例
/// ```ignore
/// let now = get_timestamp();
/// ```
pub fn get_timestamp() -> i64 {
    unsafe { solosoul_get_timestamp() }
}

/// 显示通用对话框
///
/// # 参数
/// - `config_json`: 对话框配置 JSON 字符串
///
/// # 返回值
/// - `Ok(String)`: 用户选择的 JSON 结果（如 `{"selected":"japan-visa"}`）
/// - `Err(PluginError::UserDenied)`: 用户取消
/// - `Err(PluginError::TtlExpired)`: 超时
/// - `Err(PluginError::DialogFailed)`: 其他错误
///
/// # 示例
/// ```ignore
/// let config = r#"{"title":"选择","type":"radio_list","items":[{"id":"a","label":"A"}]}"#;
/// let result = show_dialog(config).expect("对话框失败");
/// ```
pub fn show_dialog(config_json: &str) -> Result<String, PluginError> {
    const INITIAL_CAP: usize = 4096;
    let mut buf: [MaybeUninit<u8>; INITIAL_CAP] = [MaybeUninit::uninit(); INITIAL_CAP];

    let code = unsafe {
        solosoul_show_dialog(
            config_json.as_ptr(),
            config_json.len(),
            buf.as_mut_ptr() as *mut u8,
            INITIAL_CAP,
        )
    };

    match code {
        0 => {}
        -2 => return Err(PluginError::UserDenied),
        -3 => return Err(PluginError::TtlExpired),
        -5 => return Err(PluginError::PermissionDenied),
        _ => return Err(PluginError::DialogFailed),
    }

    // 查找 null terminator
    let len = unsafe {
        let mut end = INITIAL_CAP;
        for (i, b) in buf.iter().enumerate() {
            if unsafe { b.assume_init() } == 0 {
                end = i;
                break;
            }
        }
        end
    };

    let bytes: Vec<u8> = buf[..len]
        .iter()
        .map(|b| unsafe { b.assume_init() })
        .collect();

    String::from_utf8(bytes).map_err(|_| PluginError::Unknown)
}

// ============================================================================
// Phase 3: 数据结构树查询
// ============================================================================

/// 获取用户数据结构树（元数据级别）
///
/// 返回 JSON 字符串，包含页面 → 分区 → 字段的元数据（不包含字段值）。
///
/// # 示例
/// ```ignore
/// let tree_json = get_data_structure_tree().expect("获取数据结构失败");
/// ```
pub fn get_data_structure_tree() -> Result<String, PluginError> {
    const INITIAL_CAP: usize = 65536;
    let mut buf: [MaybeUninit<u8>; INITIAL_CAP] = [MaybeUninit::uninit(); INITIAL_CAP];

    let code =
        unsafe { solosoul_get_data_structure_tree(buf.as_mut_ptr() as *mut u8, INITIAL_CAP) };

    if code != 0 {
        return Err(PluginError::from_code(code));
    }

    // 查找 null terminator
    let len = unsafe {
        let mut end = INITIAL_CAP;
        for (i, b) in buf.iter().enumerate() {
            if unsafe { b.assume_init() } == 0 {
                end = i;
                break;
            }
        }
        end
    };

    let bytes: Vec<u8> = buf[..len]
        .iter()
        .map(|b| unsafe { b.assume_init() })
        .collect();

    String::from_utf8(bytes).map_err(|_| PluginError::Unknown)
}

// ============================================================================
// Phase 2: 结构化结果通道
// ============================================================================

/// 发送结构化结果（原始接口）
///
/// # 返回值
/// - `Ok(())`: 成功
/// - `Err(code)`: 错误码（见 solosoul_result 文档）
pub fn send_result_json(json: &str) -> Result<(), i32> {
    let code = unsafe { solosoul_result(json.as_ptr(), json.len()) };
    if code == 0 {
        Ok(())
    } else {
        Err(code)
    }
}

/// 发送文本结果
///
/// # 示例
/// ```ignore
/// result_text("格式化完成");
/// ```
pub fn result_text(content: &str) {
    let json = format!(r#"{{"type":"text","content":"{}"}}"#, escape_json(content));
    let _ = send_result_json(&json);
}

/// 发送键值对结果
///
/// # 示例
/// ```ignore
/// result_key_value("地址", &[("街道", "长安街1号"), ("城市", "北京")]);
/// ```
pub fn result_key_value(title: &str, pairs: &[(&str, &str)]) {
    let pairs_json: Vec<String> = pairs
        .iter()
        .map(|(k, v)| {
            format!(
                r#"{{"key":"{}","value":"{}"}}"#,
                escape_json(k),
                escape_json(v)
            )
        })
        .collect();
    let json = format!(
        r#"{{"type":"key_value","title":"{}","pairs":[{}]}}"#,
        escape_json(title),
        pairs_json.join(",")
    );
    let _ = send_result_json(&json);
}

/// 发送表格结果
///
/// # 示例
/// ```ignore
/// result_table(&["字段", "值"], &[vec!["街道", "长安街1号"], vec!["城市", "北京"]]);
/// ```
pub fn result_table(headers: &[&str], rows: &[Vec<&str>]) {
    let headers_json: Vec<String> = headers
        .iter()
        .map(|h| format!("\"{}\"", escape_json(h)))
        .collect();
    let rows_json: Vec<String> = rows
        .iter()
        .map(|row| {
            let cells: Vec<String> = row
                .iter()
                .map(|c| format!("\"{}\"", escape_json(c)))
                .collect();
            format!("[{}]", cells.join(","))
        })
        .collect();
    let json = format!(
        r#"{{"type":"table","headers":[{}],"rows":[{}]}}"#,
        headers_json.join(","),
        rows_json.join(",")
    );
    let _ = send_result_json(&json);
}

/// 发送 Markdown 结果
///
/// # 示例
/// ```ignore
/// result_markdown("**地址**：长安街1号");
/// ```
pub fn result_markdown(content: &str) {
    let json = format!(
        r#"{{"type":"markdown","content":"{}"}}"#,
        escape_json(content)
    );
    let _ = send_result_json(&json);
}

/// 从 little-endian 字节缓冲区读取 u16
fn read_u16(buf: &[MaybeUninit<u8>; 2]) -> u16 {
    let bytes = [unsafe { buf[0].assume_init() }, unsafe {
        buf[1].assume_init()
    }];
    u16::from_le_bytes(bytes)
}

/// 从 little-endian 字节缓冲区读取 u32
fn read_u32(buf: &[MaybeUninit<u8>; 4]) -> u32 {
    let bytes = [
        unsafe { buf[0].assume_init() },
        unsafe { buf[1].assume_init() },
        unsafe { buf[2].assume_init() },
        unsafe { buf[3].assume_init() },
    ];
    u32::from_le_bytes(bytes)
}

/// 简单的 JSON 字符串转义（处理双引号和反斜杠）
fn escape_json(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\u{0008}' => result.push_str("\\b"),
            '\u{000C}' => result.push_str("\\f"),
            c if c < '\u{0020}' => result.push_str(&format!("\\u{:04x}", c as u32)),
            c => result.push(c),
        }
    }
    result
}

// ============================================================================
// 辅助类型
// ============================================================================

/// 插件入口函数签名
///
/// 插件必须导出 `#[no_mangle] pub extern "C" fn run() -> i32`
///
/// # 返回值
/// - `0`: 成功
/// - 非零: 插件自定义错误码
#[allow(dead_code)]
pub type PluginMain = extern "C" fn() -> i32;
