use std::collections::BTreeSet;
use std::iter::Peekable;
use std::str::Chars;

use crate::evidence::EvidenceRecord;
use crate::file_role;

const SOURCE: &str = "stability";

pub fn detect_stability(
    path: &str,
    before: Option<&str>,
    after: Option<&str>,
    added_lines: &BTreeSet<u32>,
    touched_lines: &BTreeSet<u32>,
) -> Vec<EvidenceRecord> {
    if !path.ends_with(".rs") || !file_role::classify(path).is_prod() {
        return Vec::new();
    }

    let Some(after) = after else {
        return Vec::new();
    };

    let mut records = Vec::new();

    detect_ffi_boundary(
        path,
        before.unwrap_or_default(),
        after,
        touched_lines,
        &mut records,
    );
    detect_public_api_changes(
        path,
        before.unwrap_or_default(),
        after,
        touched_lines,
        &mut records,
    );
    detect_added_line_risks(
        path,
        before.unwrap_or_default(),
        after,
        added_lines,
        &mut records,
    );
    detect_serde_field_changes(
        path,
        before.unwrap_or_default(),
        after,
        added_lines,
        touched_lines,
        &mut records,
    );

    records
}

fn detect_ffi_boundary(
    path: &str,
    before: &str,
    after: &str,
    touched_lines: &BTreeSet<u32>,
    records: &mut Vec<EvidenceRecord>,
) {
    if !function_context_contains_ffi_boundary(after, touched_lines)
        && !function_context_contains_ffi_boundary(before, touched_lines)
    {
        return;
    }

    records.push(EvidenceRecord::new(
        "ffi_boundary_touched",
        path,
        path,
        SOURCE,
    ));
}

fn detect_public_api_changes(
    path: &str,
    before: &str,
    after: &str,
    touched_lines: &BTreeSet<u32>,
    records: &mut Vec<EvidenceRecord>,
) {
    let before_functions = public_function_signatures(before);
    let before_items = public_item_names(before);

    for (line_no, line) in after.lines().enumerate() {
        let line_no = line_no as u32 + 1;
        if !touched_lines.contains(&line_no) {
            continue;
        }

        if let Some(name) = public_item_name(line) {
            if !before_items.contains(&name) {
                records.push(EvidenceRecord::new("public_api_added", path, name, SOURCE));
                continue;
            }
        }

        let Some((name, signature)) = public_function_signature(line) else {
            continue;
        };

        if before_functions
            .get(&name)
            .is_some_and(|before_signature| before_signature != &signature)
        {
            records.push(EvidenceRecord::new(
                "public_api_signature_changed",
                path,
                name,
                SOURCE,
            ));
        }
    }
}

fn detect_added_line_risks(
    path: &str,
    before: &str,
    after: &str,
    added_lines: &BTreeSet<u32>,
    records: &mut Vec<EvidenceRecord>,
) {
    for (line_no, line) in after.lines().enumerate() {
        let line_no = line_no as u32 + 1;
        if !added_lines.contains(&line_no) {
            continue;
        }
        if line_is_in_test_context(after, line_no) {
            continue;
        }

        let code = strip_line_comment(line).trim();
        if code.is_empty() {
            continue;
        }
        let code_without_strings = strip_string_literals(code);
        let existed_before = code_existed_before(before, code);

        if !existed_before && code_without_strings.contains("unsafe") {
            records.push(EvidenceRecord::new(
                "unsafe_added",
                path,
                format!("line {line_no}"),
                SOURCE,
            ));
        }

        if !existed_before
            && (code_without_strings.contains(".unwrap(")
                || code_without_strings.contains(".expect(")
                || code_without_strings.contains("panic!("))
        {
            records.push(EvidenceRecord::new(
                "panic_like_added",
                path,
                format!("line {line_no}"),
                SOURCE,
            ));
        }

        if is_storage_or_migration_path(path) && contains_storage_migration_sql(code) {
            records.push(EvidenceRecord::new(
                "storage_migration_touched",
                path,
                format!("line {line_no}"),
                SOURCE,
            ));
        }
    }
}

fn detect_serde_field_changes(
    path: &str,
    before: &str,
    after: &str,
    added_lines: &BTreeSet<u32>,
    touched_lines: &BTreeSet<u32>,
    records: &mut Vec<EvidenceRecord>,
) {
    let before_fields = serde_public_fields(before);

    for field in serde_public_fields(after) {
        if !added_lines.contains(&field.line_no) && !touched_lines.contains(&field.line_no) {
            continue;
        }

        let before_signature = before_fields
            .iter()
            .find(|before_field| {
                before_field.struct_name == field.struct_name && before_field.name == field.name
            })
            .map(|before_field| before_field.signature.as_str());

        if before_signature != Some(field.signature.as_str()) {
            records.push(EvidenceRecord::new(
                "protocol_shape_changed",
                path,
                format!("{}::{}", field.struct_name, field.name),
                SOURCE,
            ));
        }
    }
}

fn public_function_signatures(text: &str) -> std::collections::BTreeMap<String, String> {
    text.lines()
        .filter_map(public_function_signature)
        .collect::<std::collections::BTreeMap<_, _>>()
}

fn public_item_names(text: &str) -> std::collections::BTreeSet<String> {
    text.lines().filter_map(public_item_name).collect()
}

fn public_item_name(line: &str) -> Option<String> {
    let signature = line.split('{').next()?.trim();
    let rest = signature
        .strip_prefix("pub struct ")
        .or_else(|| signature.strip_prefix("pub enum "))
        .or_else(|| signature.strip_prefix("pub trait "))
        .or_else(|| signature.strip_prefix("pub type "))?;
    let name = rest
        .split(|ch: char| ch == '(' || ch == '<' || ch == '=' || ch.is_whitespace())
        .next()
        .unwrap_or_default()
        .trim()
        .trim_end_matches(';');
    (!name.is_empty()).then(|| name.to_string())
}

fn public_function_signature(line: &str) -> Option<(String, String)> {
    let signature = line.split('{').next()?.trim();
    let rest = signature.strip_prefix("pub fn ")?;
    let name = rest
        .split_once('(')
        .map(|(name, _)| name)
        .unwrap_or(rest)
        .trim();

    if name.is_empty() {
        return None;
    }

    Some((name.to_string(), normalize_space(signature)))
}

fn contains_storage_migration_sql(line: &str) -> bool {
    let upper = line.to_ascii_uppercase();
    [
        "ALTER TABLE",
        "CREATE TABLE",
        "DROP TABLE",
        "CREATE INDEX",
        "DROP INDEX",
        "CREATE UNIQUE INDEX",
    ]
    .iter()
    .any(|keyword| upper.contains(keyword))
}

fn is_storage_or_migration_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("storage") || lower.contains("store") || lower.contains("migration")
}

#[derive(Debug, PartialEq, Eq)]
struct SerdeField {
    struct_name: String,
    name: String,
    signature: String,
    line_no: u32,
}

fn serde_public_fields(text: &str) -> Vec<SerdeField> {
    let mut fields = Vec::new();
    let mut pending_serde = false;
    let mut current_struct: Option<(String, bool)> = None;

    for (line_no, line) in text.lines().enumerate() {
        let line_no = line_no as u32 + 1;
        let trimmed = strip_line_comment(line).trim();

        if trimmed.starts_with("#[derive(")
            && (trimmed.contains("Serialize") || trimmed.contains("Deserialize"))
        {
            pending_serde = true;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("pub struct ") {
            let name = rest
                .split(|ch: char| ch == '{' || ch.is_whitespace())
                .next()
                .unwrap_or_default()
                .to_string();
            current_struct = (!name.is_empty()).then_some((name, pending_serde));
            pending_serde = false;
            continue;
        }

        if trimmed.starts_with("struct ") {
            current_struct = None;
            pending_serde = false;
            continue;
        }

        if trimmed.starts_with('}') {
            current_struct = None;
            pending_serde = false;
            continue;
        }

        let Some((struct_name, is_serde)) = current_struct.as_ref() else {
            continue;
        };
        if !*is_serde {
            continue;
        }

        let Some(rest) = trimmed.strip_prefix("pub ") else {
            continue;
        };
        let Some((name, _)) = rest.split_once(':') else {
            continue;
        };

        let name = name.trim();
        if name.is_empty() {
            continue;
        }

        fields.push(SerdeField {
            struct_name: struct_name.clone(),
            name: name.to_string(),
            signature: normalize_space(trimmed.trim_end_matches(',')),
            line_no,
        });
    }

    fields
}

fn strip_line_comment(line: &str) -> &str {
    line.split_once("//").map(|(code, _)| code).unwrap_or(line)
}

fn code_existed_before(before: &str, code: &str) -> bool {
    let normalized = normalize_space(code);
    before
        .lines()
        .map(strip_line_comment)
        .map(str::trim)
        .any(|line| normalize_space(line) == normalized)
}

fn strip_string_literals(line: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == 'r' && chars.peek() == Some(&'#') {
            output.push(' ');
            consume_raw_string_literal(&mut chars, &mut output);
        } else if ch == '"' {
            output.push(' ');
            consume_quoted_string_literal(&mut chars, &mut output);
        } else {
            output.push(ch);
        }
    }

    output
}

fn consume_quoted_string_literal(chars: &mut Peekable<Chars<'_>>, output: &mut String) {
    let mut escaped = false;
    for ch in chars.by_ref() {
        output.push(' ');
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            break;
        }
    }
}

fn consume_raw_string_literal(chars: &mut Peekable<Chars<'_>>, output: &mut String) {
    for ch in chars.by_ref() {
        output.push(' ');
        if ch == '"' {
            break;
        }
    }
}

fn normalize_space(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn function_context_contains_ffi_boundary(source: &str, touched_lines: &BTreeSet<u32>) -> bool {
    if touched_lines.is_empty() {
        return false;
    }
    let lines = source.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        return false;
    }
    for touched in touched_lines {
        if line_is_in_test_context(source, *touched) {
            continue;
        }
        let idx = touched.saturating_sub(1) as usize;
        if idx >= lines.len() {
            continue;
        }
        let start = idx.saturating_sub(4);
        let end = (idx + 4).min(lines.len().saturating_sub(1));
        if lines[start..=end].iter().any(|line| {
            let code = strip_line_comment(line).trim();
            code.contains("#[no_mangle]") || code.contains("extern \"C\"")
        }) {
            return true;
        }
    }
    false
}

fn line_is_in_test_context(source: &str, line_no: u32) -> bool {
    let mut depth = 0usize;
    let mut test_depth = None;
    let mut pending_test_cfg = false;

    for (idx, line) in source.lines().enumerate() {
        let current_line = idx as u32 + 1;
        let trimmed = strip_line_comment(line).trim();

        if test_depth.is_some_and(|scope_depth| depth < scope_depth) {
            test_depth = None;
        }
        if current_line == line_no {
            return test_depth.is_some() || trimmed.starts_with("#[test]");
        }

        if trimmed.starts_with("#[cfg(test)]") {
            pending_test_cfg = true;
        }
        if pending_test_cfg && trimmed.contains("mod tests") && trimmed.contains('{') {
            test_depth = Some(depth + 1);
            pending_test_cfg = false;
        } else if !trimmed.starts_with("#[cfg(test)]")
            && !trimmed.starts_with("#[cfg_attr")
            && !trimmed.is_empty()
        {
            pending_test_cfg = false;
        }

        depth = depth.saturating_add(line.matches('{').count());
        depth = depth.saturating_sub(line.matches('}').count());
    }

    false
}
