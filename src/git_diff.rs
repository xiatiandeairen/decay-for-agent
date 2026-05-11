use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitDiff {
    pub files: Vec<GitFileDiff>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitFileDiff {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub status: GitFileStatus,
    pub added_lines: Option<u32>,
    pub deleted_lines: Option<u32>,
    pub hunks: Vec<GitHunk>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitFileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<HunkLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HunkLine {
    pub kind: HunkLineKind,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HunkLineKind {
    Context,
    Addition,
    Deletion,
}

#[derive(Debug, Clone, Copy)]
struct Numstat {
    added: u32,
    deleted: u32,
}

pub fn parse_git_diff(input: &str) -> GitDiff {
    let numstats = parse_numstats(input);
    let mut files = Vec::new();
    let mut current: Option<GitFileDiff> = None;
    let mut current_hunk: Option<GitHunk> = None;

    for line in input.lines() {
        if let Some((old_path, new_path)) = parse_diff_git_header(line) {
            finish_hunk(&mut current, &mut current_hunk);
            if let Some(file) = current.take() {
                files.push(file);
            }

            let stat = numstats
                .get(&new_path)
                .or_else(|| numstats.get(&old_path))
                .copied();
            current = Some(GitFileDiff {
                old_path: Some(old_path),
                new_path: Some(new_path),
                status: GitFileStatus::Modified,
                added_lines: stat.map(|s| s.added),
                deleted_lines: stat.map(|s| s.deleted),
                hunks: Vec::new(),
            });
            continue;
        }

        let Some(file) = current.as_mut() else {
            continue;
        };

        if line == "new file mode 100644" || line.starts_with("new file mode ") {
            file.status = GitFileStatus::Added;
            file.old_path = None;
            continue;
        }

        if line == "deleted file mode 100644" || line.starts_with("deleted file mode ") {
            file.status = GitFileStatus::Deleted;
            file.new_path = None;
            continue;
        }

        if let Some(path) = line.strip_prefix("rename from ") {
            file.status = GitFileStatus::Renamed;
            file.old_path = Some(path.to_string());
            apply_numstat(file, &numstats);
            continue;
        }

        if let Some(path) = line.strip_prefix("rename to ") {
            file.status = GitFileStatus::Renamed;
            file.new_path = Some(path.to_string());
            apply_numstat(file, &numstats);
            continue;
        }

        if line.starts_with("--- ") || line.starts_with("+++ ") {
            apply_file_marker(file, line);
            apply_numstat(file, &numstats);
            continue;
        }

        if let Some(hunk) = parse_hunk_header(line) {
            finish_hunk(&mut current, &mut current_hunk);
            current_hunk = Some(hunk);
            continue;
        }

        if let Some(hunk) = current_hunk.as_mut() {
            if let Some(hunk_line) = parse_hunk_line(line) {
                hunk.lines.push(hunk_line);
            }
        }
    }

    finish_hunk(&mut current, &mut current_hunk);
    if let Some(file) = current {
        files.push(file);
    }

    GitDiff { files }
}

fn parse_numstats(input: &str) -> HashMap<String, Numstat> {
    let mut stats = HashMap::new();

    for line in input.lines() {
        let mut fields = line.splitn(3, '\t');
        let Some(added) = fields.next().and_then(|value| value.parse::<u32>().ok()) else {
            continue;
        };
        let Some(deleted) = fields.next().and_then(|value| value.parse::<u32>().ok()) else {
            continue;
        };
        let Some(path) = fields.next() else {
            continue;
        };

        let stat = Numstat { added, deleted };
        if let Some((old_path, new_path)) = parse_numstat_rename(path) {
            stats.insert(old_path, stat);
            stats.insert(new_path, stat);
        } else {
            stats.insert(path.to_string(), stat);
        }
    }

    stats
}

fn parse_numstat_rename(path: &str) -> Option<(String, String)> {
    let (old_path, new_path) = path.split_once(" => ")?;
    Some((old_path.to_string(), new_path.to_string()))
}

fn parse_diff_git_header(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("diff --git a/")?;
    let (old_path, new_path) = rest.split_once(" b/")?;
    Some((old_path.to_string(), new_path.to_string()))
}

fn apply_file_marker(file: &mut GitFileDiff, line: &str) {
    if let Some(path) = line.strip_prefix("--- ") {
        file.old_path = marker_path(path);
    } else if let Some(path) = line.strip_prefix("+++ ") {
        file.new_path = marker_path(path);
    }
}

fn marker_path(path: &str) -> Option<String> {
    if path == "/dev/null" {
        None
    } else {
        path.strip_prefix("a/")
            .or_else(|| path.strip_prefix("b/"))
            .map(|p| p.to_string())
            .or_else(|| Some(path.to_string()))
    }
}

fn apply_numstat(file: &mut GitFileDiff, numstats: &HashMap<String, Numstat>) {
    let stat = file
        .new_path
        .as_deref()
        .and_then(|path| numstats.get(path))
        .or_else(|| file.old_path.as_deref().and_then(|path| numstats.get(path)));

    if let Some(stat) = stat {
        file.added_lines = Some(stat.added);
        file.deleted_lines = Some(stat.deleted);
    }
}

fn parse_hunk_header(line: &str) -> Option<GitHunk> {
    let rest = line.strip_prefix("@@ -")?;
    let (old_range, rest) = rest.split_once(" +")?;
    let (new_range, header) = rest.split_once(" @@")?;
    let (old_start, old_lines) = parse_range(old_range)?;
    let (new_start, new_lines) = parse_range(new_range)?;

    Some(GitHunk {
        old_start,
        old_lines,
        new_start,
        new_lines,
        header: header.trim_start().to_string(),
        lines: Vec::new(),
    })
}

fn parse_range(range: &str) -> Option<(u32, u32)> {
    if let Some((start, lines)) = range.split_once(',') {
        Some((start.parse().ok()?, lines.parse().ok()?))
    } else {
        Some((range.parse().ok()?, 1))
    }
}

fn parse_hunk_line(line: &str) -> Option<HunkLine> {
    let mut chars = line.chars();
    let prefix = chars.next()?;
    let content = chars.as_str().to_string();
    let kind = match prefix {
        ' ' => HunkLineKind::Context,
        '+' => HunkLineKind::Addition,
        '-' => HunkLineKind::Deletion,
        _ => return None,
    };

    Some(HunkLine { kind, content })
}

fn finish_hunk(file: &mut Option<GitFileDiff>, hunk: &mut Option<GitHunk>) {
    if let (Some(file), Some(hunk)) = (file.as_mut(), hunk.take()) {
        file.hunks.push(hunk);
    }
}
