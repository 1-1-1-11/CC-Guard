// [INPUT]: Windows 原生桥（crate::windows）返回的 JSON 指针。
// [OUTPUT]: 公共 JSON 解码，以及身份保险库的凭据快照接口（Windows 使用文件凭据，接口为空实现）。
// [POS]: 原生桥的解码边界；不读取凭据管理器，不将正文交给 UI。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use std::ffi::{c_char, CStr};

use crate::windows::*;

pub(crate) fn decode<T: serde::de::DeserializeOwned>(ptr: *mut c_char) -> Result<T, String> {
    if ptr.is_null() {
        return Err("系统操作失败。".into());
    }
    let text = unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() };
    unsafe {
        rf_free(ptr);
    }
    let data: serde_json::Value =
        serde_json::from_str(&text).map_err(|_| "系统返回了无法识别的数据。")?;
    if let Some(error) = data["error"].as_str() {
        return Err(error.into());
    }
    serde_json::from_value(data).map_err(|_| "系统返回数据不完整。".into())
}


// ----- 只供身份保险库使用；没有可直接调用的 WebView 凭据命令 -----
pub(crate) fn identity_snapshot() -> Result<Vec<serde_json::Value>, String> {
    Ok(Vec::new())
}
pub(crate) fn identity_keys(items: &[serde_json::Value], restore: bool) -> Result<(), String> {
    let _ = (items, restore);
    Ok(())
}
