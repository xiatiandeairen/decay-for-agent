# decay 架构全貌

## 数据流

```
filter(L1 git/walk → L2 目录排除 → L3 文件类型 → L4 语言检测)
  → collector(file_scan + git_history + git_pipeline)
  → DataStore(OnceCell 拉模型，按需加载 source_files/dependencies)
  → dimension(8 个维度通过 DataStore 拉取数据)
  → profile(项目类型检测 + 加权 composite)
  → config(.decayrc + XDG 全局配置，合并策略：list 追加 / languages 覆盖)
```

## 扩展点（均为 trait + 注册表模式）

| 扩展 | trait | 注册处 | 需改框架？ |
|------|-------|--------|-----------|
| 新维度 | Dimension | all_dimensions() | 否 |
| 新采集器 | Collector | all_collectors() | 否 |
| 新过滤层 | FilterStage | run_pipeline() | 否 |
| 新 git 过滤 | GitFilterStage | git_pipeline::run_pipeline() | 否 |
| 新数据源 | — | DataStore 加 OnceCell + getter | 否 |

## 8 维度

structural / complexity / fragility（DB-only）+ maintainability / observability / quality_assurance / reliability / performance（file-based，通过 DataStore.source_files() 缓存共享）
