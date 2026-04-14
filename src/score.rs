use anyhow::{Context, Result};
use log::debug;
use rusqlite::Connection;

// --- Structural thresholds ---
const FILE_COUNT_WARN: i64 = 500;
const FILE_COUNT_CRIT: i64 = 1000;
const DEPTH_WARN: i64 = 5;
const DEPTH_CRIT: i64 = 8;
const TOP_DIRS_WARN: i64 = 15;

// --- Complexity thresholds ---
/// ~500 lines of code
const LARGE_FILE_BYTES: i64 = 15360;
const LARGE_RATIO_WARN: f64 = 0.2;
const LARGE_RATIO_CRIT: f64 = 0.4;
const AVG_SIZE_WARN: f64 = 10240.0;
const MAX_SIZE_WARN: i64 = 51200;

// --- Fragility thresholds ---
const CHURN_CONCENTRATION_WARN: f64 = 0.5;
const CHURN_CONCENTRATION_CRIT: f64 = 0.7;
const MAX_CHURN_WARN: i64 = 500;

/// Compute structural health score (0-100, deduction-based).
///
/// Penalizes: too many files, deep directories, too many top-level dirs.
pub fn structural(conn: &Connection, snapshot_id: i64) -> Result<i32> {
    let mut score: i32 = 100;
    debug!("structural: file_count query starting");

    // File count
    let file_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM files WHERE snapshot_id = ?1",
            [snapshot_id],
            |row| row.get(0),
        )
        .context("failed to count files")?;

    if file_count > FILE_COUNT_CRIT {
        score -= 40;
    } else if file_count > FILE_COUNT_WARN {
        score -= 20;
    }

    // Max depth
    let max_depth: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(depth), 0) FROM files WHERE snapshot_id = ?1",
            [snapshot_id],
            |row| row.get(0),
        )
        .context("failed to get max depth")?;

    if max_depth > DEPTH_CRIT {
        score -= 30;
    } else if max_depth > DEPTH_WARN {
        score -= 15;
    }

    // Top-level directory count (depth = 1, count distinct parent dirs)
    let top_dirs: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT CASE
                WHEN INSTR(path, '/') > 0 THEN SUBSTR(path, 1, INSTR(path, '/') - 1)
                ELSE path
             END) FROM files WHERE snapshot_id = ?1",
            [snapshot_id],
            |row| row.get(0),
        )
        .context("failed to count top-level dirs")?;

    if top_dirs > TOP_DIRS_WARN {
        score -= 15;
    }

    Ok(score.max(0))
}

/// Compute complexity score (0-100, deduction-based).
///
/// Penalizes: high ratio of large files, high average size, very large max file.
pub fn complexity(conn: &Connection, snapshot_id: i64) -> Result<i32> {
    let mut score: i32 = 100;
    debug!("complexity: scoring starting");

    let file_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM files WHERE snapshot_id = ?1",
            [snapshot_id],
            |row| row.get(0),
        )
        .context("failed to count files")?;

    if file_count == 0 {
        return Ok(100);
    }

    // Large file ratio (>15KB ≈ ~500 lines)
    let large_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM files WHERE snapshot_id = ?1 AND size_bytes > ?2",
            rusqlite::params![snapshot_id, LARGE_FILE_BYTES],
            |row| row.get(0),
        )
        .context("failed to count large files")?;

    let large_ratio = large_count as f64 / file_count as f64;
    if large_ratio > LARGE_RATIO_CRIT {
        score -= 45;
    } else if large_ratio > LARGE_RATIO_WARN {
        score -= 25;
    }

    // Average file size
    let avg_size: f64 = conn
        .query_row(
            "SELECT COALESCE(AVG(size_bytes), 0) FROM files WHERE snapshot_id = ?1",
            [snapshot_id],
            |row| row.get(0),
        )
        .context("failed to get avg file size")?;

    if avg_size > AVG_SIZE_WARN {
        score -= 15;
    }

    // Max file size
    let max_size: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(size_bytes), 0) FROM files WHERE snapshot_id = ?1",
            [snapshot_id],
            |row| row.get(0),
        )
        .context("failed to get max file size")?;

    if max_size > MAX_SIZE_WARN {
        score -= 10;
    }

    Ok(score.max(0))
}

/// Compute fragility score (0-100, deduction-based).
///
/// Returns None if no git change data exists for this snapshot.
/// Penalizes: churn concentration in top 10% files, very high single-file churn.
pub fn fragility(conn: &Connection, snapshot_id: i64) -> Result<Option<i32>> {
    let file_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM git_changes WHERE snapshot_id = ?1",
            [snapshot_id],
            |row| row.get(0),
        )
        .context("failed to count git_changes")?;

    if file_count == 0 {
        return Ok(None);
    }

    let mut score: i32 = 100;
    debug!("fragility: scoring starting");

    // Total churn
    let total_churn: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(lines_added + lines_deleted), 0) FROM git_changes WHERE snapshot_id = ?1",
            [snapshot_id],
            |row| row.get(0),
        )
        .context("failed to sum churn")?;

    if total_churn == 0 {
        return Ok(Some(100));
    }

    // Top 10% files churn concentration
    let top_n = (file_count as f64 * 0.1).ceil().max(1.0) as i64;
    let top_churn: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(churn), 0) FROM (
                SELECT (lines_added + lines_deleted) as churn
                FROM git_changes WHERE snapshot_id = ?1
                ORDER BY churn DESC LIMIT ?2
            )",
            rusqlite::params![snapshot_id, top_n],
            |row| row.get(0),
        )
        .context("failed to get top churn")?;

    let concentration = top_churn as f64 / total_churn as f64;
    if concentration > CHURN_CONCENTRATION_CRIT {
        score -= 45;
    } else if concentration > CHURN_CONCENTRATION_WARN {
        score -= 25;
    }

    // Max single file churn
    let max_churn: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(lines_added + lines_deleted), 0) FROM git_changes WHERE snapshot_id = ?1",
            [snapshot_id],
            |row| row.get(0),
        )
        .context("failed to get max churn")?;

    if max_churn > MAX_CHURN_WARN {
        score -= 15;
    }

    Ok(Some(score.max(0)))
}

/// Compute composite score as equal-weight average.
///
/// If fragility is N/A, average only structural and complexity.
pub fn composite(structural: i32, complexity: i32, fragility: Option<i32>) -> i32 {
    match fragility {
        Some(f) => (structural + complexity + f) / 3,
        None => (structural + complexity) / 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                snapshot_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                depth INTEGER NOT NULL
            );
            CREATE TABLE git_changes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                snapshot_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                change_count INTEGER NOT NULL,
                lines_added INTEGER NOT NULL,
                lines_deleted INTEGER NOT NULL,
                last_modified TEXT NOT NULL
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_structural_healthy() -> Result<()> {
        let conn = setup_db();
        // Small project: 10 files, depth 2
        for i in 0..10 {
            conn.execute(
                "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (1, ?1, 1000, 2)",
                [format!("src/file{i}.rs")],
            )?;
        }
        let score = structural(&conn, 1)?;
        assert!(score > 80, "healthy project should score >80, got {score}");
        Ok(())
    }

    #[test]
    fn test_structural_unhealthy() -> Result<()> {
        let conn = setup_db();
        // Large project: 600 files, depth 9
        for i in 0..600 {
            conn.execute(
                "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (1, ?1, 1000, 9)",
                [format!("a/b/c/d/e/f/g/h/i/file{i}.rs")],
            )?;
        }
        let score = structural(&conn, 1)?;
        assert!(
            score < 60,
            "unhealthy project should score <60, got {score}"
        );
        Ok(())
    }

    #[test]
    fn test_complexity_healthy() -> Result<()> {
        let conn = setup_db();
        for i in 0..20 {
            conn.execute(
                "INSERT INTO files (snapshot_id, path, size_bytes, depth) VALUES (1, ?1, 3000, 2)",
                [format!("src/file{i}.rs")],
            )?;
        }
        let score = complexity(&conn, 1)?;
        assert!(
            score > 80,
            "healthy complexity should score >80, got {score}"
        );
        Ok(())
    }

    #[test]
    fn test_fragility_no_git() -> Result<()> {
        let conn = setup_db();
        let score = fragility(&conn, 1)?;
        assert_eq!(score, None);
        Ok(())
    }

    #[test]
    fn test_fragility_with_data() -> Result<()> {
        let conn = setup_db();
        // Spread churn evenly across 10 files
        for i in 0..10 {
            conn.execute(
                "INSERT INTO git_changes (snapshot_id, path, change_count, lines_added, lines_deleted, last_modified) VALUES (1, ?1, 5, 50, 30, '2026-04-01')",
                [format!("src/file{i}.rs")],
            )?;
        }
        let score = fragility(&conn, 1)?;
        assert!(score.unwrap() > 70, "evenly spread churn should score >70");
        Ok(())
    }

    #[test]
    fn test_composite_all_dimensions() {
        let c = composite(80, 70, Some(60));
        assert_eq!(c, 70); // (80+70+60)/3 = 70
    }

    #[test]
    fn test_composite_no_fragility() {
        let c = composite(80, 70, None);
        assert_eq!(c, 75); // (80+70)/2 = 75
    }
}
