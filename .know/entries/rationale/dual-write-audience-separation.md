# Dual-write ≠ redundancy — check audience before merging

## Why

When two fields carry similar content, the instinct is to merge them (DRY). But if they serve different consumers, merging forces one field to satisfy two audiences — degrading both.

In decay v4, `prescription` (human-friendly imperative: "split into sub-modules") and `action.reason` (agent context: "1200 files exceed threshold") looked redundant. They weren't — they had different semantics (imperative vs declarative) for different audiences (CLI reader vs agent).

## Resolution

Split into `suggestion` (human instruction, imperative) and `reason` (agent context, declarative) on Action. Removed `prescription` from Issue entirely — Issue became pure diagnostic.

## How to check

Before merging "duplicate" fields, ask:
1. Do they serve different consumers? (human vs machine, CLI vs API)
2. Are their natural writing styles different? (imperative vs declarative, summary vs detail)
3. Would merging force awkward compromise text?

If any answer is yes, keep them separate.
