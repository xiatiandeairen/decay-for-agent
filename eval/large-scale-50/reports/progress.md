# Large-scale 50 progress

Date started: 2026-05-12

## Current status

- Target accepted high-value diffs: 50
- Accepted candidates: 31
  - mxcore: 11 high-value + 1 negative control
  - open source: 20 high-value candidates pending full PR-context freeze
- Filtered: 0
- Frozen high-value cases: 5
- Analyzed: 0
- Compared: 0
- Bad cases recorded: 0

## Notes

- The corpus must not be shaped by artificial buckets.
- Medium-confidence cases are useful process notes but do not count as
  high-value validation.
- Replay must use full base/head SHAs. Short SHAs caused `baseline_not_found`
  and later wrong-repo/wrong-revision replays.
- Running decay replay must happen inside the target repo checkout. Running from
  the decay repo with mxcore SHAs produced invalid evidence.
- Temporary clones need all refs/objects when replaying commits outside the
  default branch ancestry.

## Frozen mxcore smoke set

| Case | Evidence count | Notes |
| --- | ---: | --- |
| `mxcore-dac56d9` | 10 | TLS pinning/security boundary. |
| `mxcore-eeefff5` | 3 | HTTP retry behavior; decay output is sparse. |
| `mxcore-59edd1a` | 7 | Transfer hardening; focused complexity/history signals. |
| `mxcore-d8ef46c` | 72 | Storage/FFI/security boundary; likely evidence overload case. |
| `mxcore-1bab54c` | 3 | Cancel-safe frame read; focused complexity/stability/history signals. |

## Immediate next step

Freeze full PR context for the strongest open-source candidates with lightweight
`gh pr view` calls, then run decay replay for a smaller subset before scaling.
