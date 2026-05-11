use std::collections::BTreeSet;

use crate::evidence::EvidenceRecord;

const SOURCE: &str = "stability";

pub fn detect_stability(
    path: &str,
    before: Option<&str>,
    after: Option<&str>,
    added_lines: &BTreeSet<u32>,
    touched_lines: &BTreeSet<u32>,
) -> Vec<EvidenceRecord> {
    if !path.ends_with(".rs") {
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
    detect_added_line_risks(path, after, added_lines, &mut records);
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
    if !function_context_contains(after, touched_lines, &["#[no_mangle]", "extern \"C\""])
        && !function_context_contains(before, touched_lines, &["#[no_mangle]", "extern \"C\""])
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

    for (line_no, line) in after.lines().enumerate() {
        let line_no = line_no as u32 + 1;
        if !touched_lines.contains(&line_no) {
            continue;
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
    after: &str,
    added_lines: &BTreeSet<u32>,
    records: &mut Vec<EvidenceRecord>,
) {
    for (line_no, line) in after.lines().enumerate() {
        let line_no = line_no as u32 + 1;
        if !added_lines.contains(&line_no) {
            continue;
        }

        let code = strip_line_comment(line).trim();
        if code.is_empty() {
            continue;
        }

        if code.contains("unsafe") {
            records.push(EvidenceRecord::new(
                "unsafe_added",
                path,
                format!("line {line_no}"),
                SOURCE,
            ));
        }

        if code.contains(".unwrap(") || code.contains(".expect(") || code.contains("panic!(") {
            records.push(EvidenceRecord::new(
                "panic_like_added",
                path,
                format!("line {line_no}"),
                SOURCE,
            ));
        }

        if contains_storage_migration_sql(code) {
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

fn normalize_space(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn function_context_contains(
    source: &str,
    touched_lines: &BTreeSet<u32>,
    needles: &[&str],
) -> bool {
    if touched_lines.is_empty() {
        return false;
    }
    let lines = source.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        return false;
    }
    for touched in touched_lines {
        let idx = touched.saturating_sub(1) as usize;
        let start = idx.saturating_sub(4);
        let end = (idx + 4).min(lines.len().saturating_sub(1));
        let context = lines[start..=end].join("\n");
        if needles.iter().any(|needle| context.contains(needle)) {
            return true;
        }
    }
    false
}
