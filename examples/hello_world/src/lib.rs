//! Hello World — SoloSoul 插件最小示例
//!
//! 本插件不请求任何用户数据，仅调用 `get_timestamp` 和 `log`，
//! 用于验证 Wasmtime 沙盒 + Host Functions 的最小链路。
//!
//! 编译：
//! ```bash
//! cargo build --target wasm32-wasi --release
//! ```
//! 产物：`target/wasm32-wasi/release/hello_world.wasm`
//! 重命名：`cp target/wasm32-wasi/release/hello_world.wasm plugin.wasm`

use solosoul_plugin_sdk::{get_timestamp, log_info};

/// 插件入口函数
///
/// SoloSoul Host 通过 `run` 符号调用此函数。
/// 返回值 `0` 表示成功，非零表示插件自定义错误码。
#[no_mangle]
pub extern "C" fn run() -> i32 {
    let now = get_timestamp();
    log_info(&format!("Hello from SoloSoul plugin! Timestamp: {}", now));
    0
}
