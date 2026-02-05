use crate::error::IdlcError;
use crate::generate_from_source;
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[derive(Deserialize)]
struct GenerateRequest {
    lang: String,
    idl: String,
    #[serde(default)]
    props: HashMap<String, serde_json::Value>,
}

#[derive(serde::Serialize)]
struct GenerateResponse {
    files: Vec<GeneratedFile>,
}

#[derive(serde::Serialize)]
struct GeneratedFile {
    path: String,
    content: String,
}

fn json_error(err: impl ToString) -> *mut c_char {
    let msg = serde_json::json!({
        "error": err.to_string(),
    });
    CString::new(msg.to_string())
        .unwrap_or_else(|_| CString::new("{\"error\":\"invalid error\"}").unwrap())
        .into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn xidlc_generate_json(input: *const c_char) -> *mut c_char {
    if input.is_null() {
        return json_error("input is null");
    }
    let cstr = unsafe { CStr::from_ptr(input) };
    let input_str = match cstr.to_str() {
        Ok(value) => value,
        Err(err) => return json_error(format!("invalid utf-8 input: {err}")),
    };
    let req: GenerateRequest = match serde_json::from_str(input_str) {
        Ok(value) => value,
        Err(err) => return json_error(format!("invalid json: {err}")),
    };

    let files = match generate_from_source(&req.lang, &req.idl, req.props) {
        Ok(files) => files,
        Err(err) => return json_error(err.to_string()),
    };

    let out = GenerateResponse {
        files: files
            .into_iter()
            .map(|file| GeneratedFile {
                path: file.path().to_string(),
                content: file.content().to_string(),
            })
            .collect(),
    };

    match serde_json::to_string(&out) {
        Ok(text) => CString::new(text)
            .unwrap_or_else(|_| CString::new("{\"error\":\"invalid output\"}").unwrap())
            .into_raw(),
        Err(err) => json_error(format!("serialize error: {err}")),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn xidlc_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

#[allow(dead_code)]
fn _assert_error_send_sync() {
    fn _assert(_: IdlcError) {}
}
