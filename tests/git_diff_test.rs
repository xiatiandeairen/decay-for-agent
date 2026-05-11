use decay::git_diff::{parse_git_diff, GitFileStatus, HunkLineKind};

#[test]
fn parses_file_status_and_numstat_for_added_modified_deleted_and_renamed_files() {
    let diff = "\
2\t0\tsrc/new.rs
1\t1\tsrc/lib.rs
0\t3\tsrc/old.rs
4\t2\tsrc/old name.rs => src/new name.rs

diff --git a/src/new.rs b/src/new.rs
new file mode 100644
--- /dev/null
+++ b/src/new.rs
@@ -0,0 +1,2 @@
+fn added() {}
+fn second() {}
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,3 @@
 fn keep() {}
-fn old() {}
+fn new() {}
diff --git a/src/old.rs b/src/old.rs
deleted file mode 100644
--- a/src/old.rs
+++ /dev/null
@@ -1,3 +0,0 @@
-fn removed() {}
-fn gone() {}
-fn done() {}
diff --git a/src/old name.rs b/src/new name.rs
similarity index 80%
rename from src/old name.rs
rename to src/new name.rs
--- a/src/old name.rs
+++ b/src/new name.rs
@@ -1,2 +1,4 @@
 fn same() {}
-fn before() {}
+fn after() {}
+fn extra_one() {}
+fn extra_two() {}
";

    let parsed = parse_git_diff(diff);

    assert_eq!(parsed.files.len(), 4);

    assert_eq!(parsed.files[0].status, GitFileStatus::Added);
    assert_eq!(parsed.files[0].old_path, None);
    assert_eq!(parsed.files[0].new_path.as_deref(), Some("src/new.rs"));
    assert_eq!(parsed.files[0].added_lines, Some(2));
    assert_eq!(parsed.files[0].deleted_lines, Some(0));

    assert_eq!(parsed.files[1].status, GitFileStatus::Modified);
    assert_eq!(parsed.files[1].old_path.as_deref(), Some("src/lib.rs"));
    assert_eq!(parsed.files[1].new_path.as_deref(), Some("src/lib.rs"));
    assert_eq!(parsed.files[1].added_lines, Some(1));
    assert_eq!(parsed.files[1].deleted_lines, Some(1));

    assert_eq!(parsed.files[2].status, GitFileStatus::Deleted);
    assert_eq!(parsed.files[2].old_path.as_deref(), Some("src/old.rs"));
    assert_eq!(parsed.files[2].new_path, None);
    assert_eq!(parsed.files[2].added_lines, Some(0));
    assert_eq!(parsed.files[2].deleted_lines, Some(3));

    assert_eq!(parsed.files[3].status, GitFileStatus::Renamed);
    assert_eq!(parsed.files[3].old_path.as_deref(), Some("src/old name.rs"));
    assert_eq!(parsed.files[3].new_path.as_deref(), Some("src/new name.rs"));
    assert_eq!(parsed.files[3].added_lines, Some(4));
    assert_eq!(parsed.files[3].deleted_lines, Some(2));
}

#[test]
fn parses_unified_diff_hunks_with_line_kinds() {
    let diff = "\
3\t2\tsrc/lib.rs

diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -10,5 +10,6 @@ fn alpha() {
 let kept = true;
-let old = 1;
+let new = 1;
+let inserted = 2;
 tail();
@@ -30,2 +31,2 @@ fn beta() {
-old_call();
+new_call();
 }
";

    let parsed = parse_git_diff(diff);
    let file = &parsed.files[0];

    assert_eq!(file.hunks.len(), 2);

    let first = &file.hunks[0];
    assert_eq!(first.old_start, 10);
    assert_eq!(first.old_lines, 5);
    assert_eq!(first.new_start, 10);
    assert_eq!(first.new_lines, 6);
    assert_eq!(first.header, "fn alpha() {");
    assert_eq!(first.lines[0].kind, HunkLineKind::Context);
    assert_eq!(first.lines[0].content, "let kept = true;");
    assert_eq!(first.lines[1].kind, HunkLineKind::Deletion);
    assert_eq!(first.lines[1].content, "let old = 1;");
    assert_eq!(first.lines[2].kind, HunkLineKind::Addition);
    assert_eq!(first.lines[2].content, "let new = 1;");

    let second = &file.hunks[1];
    assert_eq!(second.old_start, 30);
    assert_eq!(second.old_lines, 2);
    assert_eq!(second.new_start, 31);
    assert_eq!(second.new_lines, 2);
    assert_eq!(second.header, "fn beta() {");
}

#[test]
fn parses_single_line_hunk_ranges_and_paths_with_spaces() {
    let diff = "\
1\t1\tsrc/file with space.rs

diff --git a/src/file with space.rs b/src/file with space.rs
--- a/src/file with space.rs
+++ b/src/file with space.rs
@@ -7 +7 @@ fn spaced() {
-old();
+new();
";

    let parsed = parse_git_diff(diff);
    let file = &parsed.files[0];
    let hunk = &file.hunks[0];

    assert_eq!(file.new_path.as_deref(), Some("src/file with space.rs"));
    assert_eq!(file.status, GitFileStatus::Modified);
    assert_eq!(hunk.old_start, 7);
    assert_eq!(hunk.old_lines, 1);
    assert_eq!(hunk.new_start, 7);
    assert_eq!(hunk.new_lines, 1);
}
