# 分层打分系统 技术方案

## 1. 背景

PRD 要求自动检测项目类型，按类型适配阈值和权重。需要新增项目类型检测模块和 ScoreProfile 数据结构，并扩展 Dimension trait 使其可访问 profile。

## 2. 方案

### 2.1 项目类型

```rust
// src/profile.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectType {
    Cli,
    WebService,
    Library,
    MobileApp,
    Monorepo,
    Generic,
}
```

### 2.2 自动检测

基于文件特征检测，按优先级匹配：

| 项目类型 | 检测信号 |
|---------|---------|
| MobileApp | `Info.plist` 或 `AndroidManifest.xml` 存在 |
| Monorepo | 多个 `Cargo.toml` 或 `package.json`（workspace） |
| WebService | 依赖文件含 actix/axum/express/flask/django/gin/rocket |
| Library | `lib.rs` 为入口且无 `main.rs`，或 `__init__.py` 无 `__main__.py` |
| Cli | `main.rs`/`main.py` 存在 + 依赖含 clap/argparse/cobra |
| Generic | 以上都不匹配 |

检测只读文件系统（不执行命令），O(1) 文件检查 + 依赖文件 grep。

```rust
pub fn detect(project_path: &Path) -> ProjectType
```

### 2.3 ScoreProfile

```rust
pub struct ScoreProfile {
    pub project_type: ProjectType,
    /// Dimension weights for composite score (dimension_name → weight 0.0-1.0).
    pub weights: HashMap<String, f64>,
}

impl ScoreProfile {
    pub fn for_type(pt: ProjectType) -> Self { ... }
}
```

权重表（v3 初始值，3 个现有维度）：

| 维度 | CLI | WebService | Library | MobileApp | Monorepo | Generic |
|------|-----|-----------|---------|-----------|----------|---------|
| structural | 0.30 | 0.25 | 0.30 | 0.25 | 0.35 | 0.33 |
| complexity | 0.40 | 0.30 | 0.40 | 0.35 | 0.30 | 0.34 |
| fragility | 0.30 | 0.45 | 0.30 | 0.40 | 0.35 | 0.33 |

权重归一化：composite = Σ(score_i × weight_i) / Σ(weight_i)，跳过 score=None 的维度。

新增维度（M4-M8）时各自在 ScoreProfile 中添加权重行。

### 2.4 Dimension trait 扩展

当前 score() 签名：`fn score(&self, conn, snapshot_id) -> Result<Option<i32>>`

两种扩展方式：
- **A) 传 profile 参数**：`fn score(&self, conn, snapshot_id, profile: &ScoreProfile) -> ...`
- **B) 维度内部不感知 profile**：阈值仍硬编码在维度内，profile 只影响 composite 权重

推荐 B：v3 现有维度的阈值不需要按项目类型变化，profile 只用于权重。未来 M4-M8 如果需要按项目类型调阈值，在各自的 score() 内部读取 profile。到那时再扩展 trait 签名。

### 2.5 run.rs 变更

```rust
// 检测项目类型
let project_type = profile::detect(&project_path);
let score_profile = profile::ScoreProfile::for_type(project_type);
debug!("detected project type: {project_type:?}");

// ... evaluate dimensions ...

// Composite 用 profile 权重
let comp = score_profile.weighted_composite(&scores);
```

### 2.6 JSON 输出变更

Report struct 新增字段：

```rust
pub struct Report {
    pub project_type: profile::ProjectType,
    // ... existing fields ...
}
```

输出：
```json
{
  "project_type": "cli",
  "scores": { ... },
  ...
}
```

## 3. 文件变更清单

| Action | File | 变更 |
|--------|------|------|
| create | `src/profile.rs` | ProjectType + detect() + ScoreProfile |
| modify | `src/main.rs` | 添加 `mod profile` |
| modify | `src/run.rs` | 调用 detect()，composite 改用加权，Report 新增 project_type |

不修改：dimension trait 签名、collector、db、scan、git

## 4. 关键决策

| 决策 | 结论 | 理由 |
|------|------|------|
| 检测方式 | 文件特征，不执行命令 | 快速、无副作用 |
| profile 影响范围 | 仅 composite 权重 | 现有维度阈值无需按项目类型变，最小改动 |
| Dimension trait | 不扩展签名 | 避免破坏所有维度实现，按需在各维度内部处理 |
| 权重配置 | 代码硬编码 | v3 不做用户可配，先用合理默认值 |
| 检测 fallback | Generic（等权重） | 误检测比不检测安全 |

## 5. 迭代记录

- 2026-04-14: 初始方案
