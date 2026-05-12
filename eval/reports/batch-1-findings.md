# Batch 1 Findings

## Scope

- cases: 10
- repos: `hyperium/hyper`, `BurntSushi/ripgrep`
- judged: 10
- status: first validation batch, not final product conclusion

## Metrics

- precision_at_top3: 45.45%
- actionable_pr_rate: 50.00%
- negative_control_silence_rate: 66.67%
- useful_evidence_count: 18
- plausible_evidence_count: 13
- noise_evidence_count: 15
- missed_high_risk_count: 3
- noise_top3_count: 6
- history_only_false_positive_count: 2

## Useful Signals

- Parser/grammar complexity worked well on `ripgrep-3210`: top findings pointed directly to POSIX class parser and conversion functions.
- Semantic constant/default diff worked well on `ripgrep-3224`: `--` added to decompression command arrays was accurately captured.
- Complexity evidence was useful on `hyper-3936` because it pointed at HTTP/1 connection/parser functions touched by a security/protocol behavior change.
- Negative controls were correctly silent for docs-only `ripgrep-3219` and comment-only `ripgrep-3227`.

## Missed Risks

- `hyper-3621`: missed Host header userinfo/security semantics. Decay saw generic panic/default/blast evidence but not the actual protocol header risk.
- `hyper-3936`: missed explicit HTTP/1.0 keep-alive config/API/security-advisory semantics, though complexity pointed at relevant code.
- `hyper-3971`: missed public API design/header casing validation risk, despite PR discussion being centered on API design.

## Noise

- Moved/refactored panic-like code ranked high in `hyper-3606`.
- Existing high-complexity functions touched by cfg rewrites dominated `hyper-3865`, while the useful build-script concern ranked lower.
- Mechanical clippy changes in `ripgrep-3230` produced blast/mixed-concerns noise.
- History-only findings in `hyper-3971` were not useful without direct connection to the API/header-casing risk.

## Next Evidence Priorities

1. Protocol/header semantic evidence: Host, Connection, keep-alive, HTTP version behavior, header casing.
2. Public API addition/change evidence beyond simple `pub fn` signature changes.
3. Refactor/move-aware suppression for panic-like additions and touched complexity.
4. Mechanical lint/refactor classifier to keep low-risk cleanup quiet.
5. Better ranking: semantic protocol/API/build evidence should outrank generic complexity or history.
