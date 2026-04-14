# JSON 输出 技术方案

## 1. 背景

`--json` flag 输出结构化 JSON。

## 2. 方案

Cli struct 加 `--json` bool flag。serde_json 序列化 Report struct。现有数据结构加 `#[derive(Serialize)]`。

### JSON 结构

```json
{
  "snapshot_id": 1,
  "scores": { "structural": 85, "complexity": 100, "fragility": 60, "composite": 81 },
  "trend": { "structural": "+3", "complexity": "0", "fragility": "N/A", "composite": "-1" },
  "issues": [{ "level": "critical", "category": "fragility", "message": "...", "prescription": "..." }],
  "scan": { "file_count": 159, "dir_count": 62, "max_depth": 5 },
  "git": { "total_commits": 7, "files_analyzed": 30 }
}
```

## 3. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 序列化库 | serde_json | 标准做法 |
| 输出结构 | 单个 Report 对象 | 一次序列化，干净 |

## 4. 迭代记录

- 2026-04-14: 初始方案
