//! 附件水印添加插件
//!
//! 工作流程：
//! 1. 前端在自定义 UI 中完成水印配置与附件选择，通过 params 传入：
//!    - watermarkConfig: JSON 字符串（SDK WatermarkConfig 格式）
//!    - selectedAttachments: JSON 数组（每项含 objectId/attachmentId/fileName/mimeType）
//!    - outputDir: 用户指定的输出目录
//! 2. 插件为每个附件调用 Host 复制副本、添加水印、再复制到输出目录。
//! 3. 插件通过结构化结果返回处理列表，前端展示下载/预览按钮。

use serde::{Deserialize, Serialize};
use solosoul_plugin_sdk as sdk;

/// 前端传入的附件选择项
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SelectedAttachment {
    object_id: String,
    attachment_id: String,
    file_name: String,
    mime_type: String,
}

/// 单条处理结果
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WatermarkItem {
    object_id: String,
    attachment_id: String,
    file_name: String,
    mime_type: String,
    output_path: String,
}

/// 插件结果载荷
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WatermarkResult {
    #[serde(rename = "type")]
    result_type: String,
    output_dir: String,
    items: Vec<WatermarkItem>,
}

#[no_mangle]
pub extern "C" fn run() -> i32 {
    if let Err(e) = run_inner() {
        sdk::log_error(&format!("水印插件运行失败: {:?}", e));
        return -1;
    }
    0
}

fn run_inner() -> Result<(), Box<dyn std::error::Error>> {
    let watermark_config_json = sdk::get_param("watermarkConfig").unwrap_or_default();
    if watermark_config_json.is_empty() {
        sdk::log_error("缺少水印配置参数 watermarkConfig");
        return Err("缺少水印配置".into());
    }

    let selected_json = sdk::get_param("selectedAttachments").unwrap_or_default();
    if selected_json.is_empty() {
        sdk::log_error("缺少已选择附件参数 selectedAttachments");
        return Err("缺少已选择附件".into());
    }
    let selected: Vec<SelectedAttachment> = serde_json::from_str(&selected_json)?;
    if selected.is_empty() {
        sdk::log_warn("未选择任何附件");
        return Ok(());
    }

    let output_dir = sdk::get_param("outputDir").unwrap_or_default();
    if output_dir.is_empty() {
        sdk::log_error("缺少输出目录参数 outputDir");
        return Err("缺少输出目录".into());
    }

    sdk::log_info(&format!("开始为 {} 个附件添加水印", selected.len()));

    let mut items: Vec<WatermarkItem> = Vec::with_capacity(selected.len());

    for att in &selected {
        sdk::log_info(&format!(
            "处理附件: {} (对象 {})",
            att.file_name, att.object_id
        ));

        let input_path = match sdk::prepare_attachment_copy(&att.object_id, &att.attachment_id) {
            Ok(p) => p,
            Err(e) => {
                sdk::log_error(&format!(
                    "复制附件 {} 失败: {:?}",
                    att.file_name, e
                ));
                continue;
            }
        };

        // 输出路径放在工作区，使用 .watermarked 后缀避免覆盖输入
        let workspace_output = add_suffix_before_ext(&input_path, ".watermarked");

        let is_pdf = att.mime_type.eq_ignore_ascii_case("application/pdf")
            || att.file_name.to_ascii_lowercase().ends_with(".pdf");

        let watermark_result = if is_pdf {
            sdk::pdf_watermark(&input_path, &workspace_output, &parse_config(&watermark_config_json)?)
        } else {
            sdk::image_watermark(&input_path, &workspace_output, &parse_config(&watermark_config_json)?)
        };

        if let Err(e) = watermark_result {
            sdk::log_error(&format!(
                "为 {} 添加水印失败: {:?}",
                att.file_name, e
            ));
            continue;
        }

        let final_path = match sdk::copy_output_file(&workspace_output, &att.file_name) {
            Ok(p) => p,
            Err(e) => {
                sdk::log_error(&format!(
                    "复制结果 {} 到输出目录失败: {:?}",
                    att.file_name, e
                ));
                continue;
            }
        };

        items.push(WatermarkItem {
            object_id: att.object_id.clone(),
            attachment_id: att.attachment_id.clone(),
            file_name: att.file_name.clone(),
            mime_type: att.mime_type.clone(),
            output_path: final_path,
        });

        sdk::log_info(&format!("{} 已添加水印", att.file_name));
    }

    sdk::log_info(&format!("水印处理完成，成功 {} / 总计 {}", items.len(), selected.len()));

    let result = WatermarkResult {
        result_type: "watermark_result".to_string(),
        output_dir,
        items,
    };
    let _ = sdk::send_result_json(&serde_json::to_string(&result)?);

    Ok(())
}

fn parse_config(json: &str) -> Result<sdk::WatermarkConfig, Box<dyn std::error::Error>> {
    // SDK 的 WatermarkConfig 与 Host 侧 JSON 字段保持一致（camelCase）
    let parsed: serde_json::Value = serde_json::from_str(json)?;
    Ok(sdk::WatermarkConfig {
        text: parsed["text"].as_str().unwrap_or("SoloSoul").to_string(),
        font_size: parsed["fontSize"].as_f64().unwrap_or(72.0) as f32,
        color: parse_color(&parsed["color"]),
        opacity: parsed["opacity"].as_f64().unwrap_or(0.3) as f32,
        angle: parsed["angle"].as_f64().unwrap_or(-45.0) as f32,
        position: parse_position(parsed["position"].as_str().unwrap_or("center")),
        tile: parsed["tile"].as_bool().unwrap_or(false),
        margin_x: parsed["marginX"].as_i64().unwrap_or(0) as i32,
        margin_y: parsed["marginY"].as_i64().unwrap_or(0) as i32,
    })
}

fn parse_color(value: &serde_json::Value) -> [u8; 3] {
    if let Some(arr) = value.as_array() {
        [
            arr.get(0).and_then(|v| v.as_u64()).unwrap_or(128) as u8,
            arr.get(1).and_then(|v| v.as_u64()).unwrap_or(128) as u8,
            arr.get(2).and_then(|v| v.as_u64()).unwrap_or(128) as u8,
        ]
    } else {
        [128, 128, 128]
    }
}

fn parse_position(s: &str) -> sdk::WatermarkPosition {
    match s {
        "topLeft" => sdk::WatermarkPosition::TopLeft,
        "topRight" => sdk::WatermarkPosition::TopRight,
        "bottomLeft" => sdk::WatermarkPosition::BottomLeft,
        "bottomRight" => sdk::WatermarkPosition::BottomRight,
        "tile" => sdk::WatermarkPosition::Tile,
        _ => sdk::WatermarkPosition::Center,
    }
}

/// 在扩展名之前插入后缀；没有扩展名则追加到末尾。
fn add_suffix_before_ext(path: &str, suffix: &str) -> String {
    if let Some(dot) = path.rfind('.') {
        let (base, ext) = path.split_at(dot);
        format!("{}{}{}", base, suffix, ext)
    } else {
        format!("{}{}", path, suffix)
    }
}
