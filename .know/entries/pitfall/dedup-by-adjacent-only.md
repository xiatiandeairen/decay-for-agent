# dedup_by only removes adjacent duplicates

## Symptoms

`Vec::dedup_by` called on an unsorted vector silently misses non-adjacent duplicates. No compile error, no runtime panic — just wrong results.

## Root cause

Rust's `dedup_by` (like C++ `std::unique`) assumes the vector is already sorted so identical elements are adjacent. Without sorting first, elements with the same key but separated by different elements survive dedup.

## Lesson

Always sort before dedup, or encapsulate both into a single function (`collect_sorted`) so the ordering invariant cannot be violated by callers. Comments like "// sort first, then dedup" are insufficient — the next editor may reorder them.
