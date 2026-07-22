use std::path::Path;
use wasm_bindgen::prelude::*;

use crate::current_lint;
use crate::hover::hover_at as hover_at_impl;
use crate::lint::lint_source as lint_legacy_source;

#[wasm_bindgen]
pub fn lint(filename: &str, source: &str) -> JsValue {
    let path = Path::new(filename);
    let diagnostics = if current_lint::should_lint_as_current(path, source) {
        current_lint::current_lint_source(path, source).diagnostics
    } else {
        lint_legacy_source(path, source).diagnostics
    };
    let diags: Vec<JsDiagnostic> = diagnostics
        .iter()
        .map(|d| JsDiagnostic {
            line: d.line,
            column: d.column,
            severity: d.severity.to_string(),
            code: d.code.to_string(),
            message: d.message.clone(),
        })
        .collect();
    serde_wasm_bindgen::to_value(&diags).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn hover(source: &str, line: u32, col: u32) -> Option<String> {
    hover_at_impl(source, line, col)
}

#[derive(serde::Serialize)]
struct JsDiagnostic {
    line: usize,
    column: usize,
    severity: String,
    code: String,
    message: String,
}

/// Declared parameters of a program's `### Parameters` section — the run
/// surfaces derive their input forms from this (the linter is the canonical
/// dialect parser). Returns an array of
/// {name, description, required, shape, default}.
#[wasm_bindgen]
pub fn parameters(source: &str) -> JsValue {
    let params = crate::params::declared_parameters(source);
    serde_wasm_bindgen::to_value(&params).unwrap_or(JsValue::NULL)
}
