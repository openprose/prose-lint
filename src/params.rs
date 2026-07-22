//! Declared-parameter extraction for host-generated input forms.
//!
//! The linter is the canonical dialect parser, so run surfaces (web editor,
//! program pages, CLI) derive their input forms from THIS instead of
//! hand-parsing `### Parameters`. Kept deliberately structural: name, prose,
//! required-ness, declared shape, and a default when the prose expresses one.

use serde::Serialize;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct DeclaredParam {
    pub name: String,
    /// The parameter's prose (everything after `` `name`: ``, continuation
    /// lines folded in).
    pub description: String,
    /// False when the prose marks it optional or provides a default.
    pub required: bool,
    /// "array" | "object" | "number" | "boolean" | "string" — a dialect-prose
    /// heuristic ("JSON array of…", "number of…"); "string" when unstated.
    pub shape: String,
    /// Default value text when the prose says "default(s to) X".
    pub default: Option<String>,
}

fn shape_of(desc: &str) -> &'static str {
    let d = desc.to_lowercase();
    if d.contains("array") || d.contains("list of") {
        "array"
    } else if d.contains("json object") || d.contains("object of shape") || d.contains("map of") {
        "object"
    } else if d.contains("number of") || d.starts_with("number") || d.contains("integer") || d.contains("count of") {
        "number"
    } else if d.contains("boolean") || d.contains("true or false") || d.contains("true/false") {
        "boolean"
    } else {
        "string"
    }
}

fn default_of(desc: &str) -> Option<String> {
    let lower = desc.to_lowercase();
    for marker in ["defaults to ", "default: ", "default is "] {
        if let Some(idx) = lower.find(marker) {
            let tail = &desc[idx + marker.len()..];
            let end = tail
                .find(|c: char| c == '.' || c == ';' || c == ',' || c == ')')
                .unwrap_or(tail.len());
            let val = tail[..end].trim().trim_matches('`').trim_matches('"');
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

/// Extract the `### Parameters` section's declared parameters.
/// Items are `- `name`: prose` bullets; continuation lines (indented, no
/// bullet) fold into the previous item's prose. Absent section → empty vec.
pub fn declared_parameters(source: &str) -> Vec<DeclaredParam> {
    let mut params: Vec<DeclaredParam> = Vec::new();
    let mut in_section = false;
    for line in source.lines() {
        let trimmed = line.trim_end();
        if let Some(header) = trimmed.strip_prefix("### ") {
            in_section = header.trim().eq_ignore_ascii_case("parameters");
            continue;
        }
        if trimmed.starts_with("## ") || trimmed.starts_with("# ") {
            in_section = false;
            continue;
        }
        if !in_section {
            continue;
        }
        let lt = trimmed.trim_start();
        if let Some(item) = lt.strip_prefix("- ") {
            // - `name`: prose
            if let Some(rest) = item.strip_prefix('`') {
                if let Some(close) = rest.find('`') {
                    let name = &rest[..close];
                    let after = rest[close + 1..].trim_start_matches(':').trim();
                    if !name.is_empty() {
                        params.push(DeclaredParam {
                            name: name.to_string(),
                            description: after.to_string(),
                            required: true, // finalized below
                            shape: "string".to_string(),
                            default: None,
                        });
                    }
                }
            }
        } else if !lt.is_empty() {
            // continuation line — fold into previous item's prose
            if let Some(last) = params.last_mut() {
                if !last.description.is_empty() {
                    last.description.push(' ');
                }
                last.description.push_str(lt);
            }
        }
    }
    for p in params.iter_mut() {
        p.shape = shape_of(&p.description).to_string();
        p.default = default_of(&p.description);
        let lower = p.description.to_lowercase();
        p.required = p.default.is_none() && !lower.contains("optional");
    }
    params
}

#[cfg(test)]
mod tests {
    use super::*;

    const GREETING_REPORT: &str = "---\nname: greeting-report\nkind: function\n---\n\n### Parameters\n\n- `names`: JSON array of person names to greet\n\n### Returns\n\n- `report`: JSON object\n";

    #[test]
    fn extracts_greeting_report_param() {
        let p = declared_parameters(GREETING_REPORT);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].name, "names");
        assert_eq!(p[0].shape, "array");
        assert!(p[0].required);
    }

    #[test]
    fn optional_and_default_and_shapes() {
        let src = "### Parameters\n\n- `topic`: a subject to write about\n- `count`: number of items to produce. Defaults to 3.\n- `strict`: boolean, optional\n- `tags`: list of labels\n  spanning two lines\n";
        let p = declared_parameters(src);
        assert_eq!(p.len(), 4);
        assert_eq!(p[0].shape, "string");
        assert!(p[0].required);
        assert_eq!(p[1].shape, "number");
        assert_eq!(p[1].default.as_deref(), Some("3"));
        assert!(!p[1].required);
        assert_eq!(p[2].shape, "boolean");
        assert!(!p[2].required);
        assert_eq!(p[3].shape, "array");
        assert!(p[3].description.contains("spanning two lines"));
    }

    #[test]
    fn no_section_means_no_params() {
        assert!(declared_parameters("---\nname: x\n---\n\n### Returns\n\n- `y`: z\n").is_empty());
    }
}
