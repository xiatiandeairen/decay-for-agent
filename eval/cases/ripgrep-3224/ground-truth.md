# Ground Truth

## External Facts

- PR body: fixes bug where compressed filenames starting with `-` are passed to decompression tools and treated as options.
- Body states worst case could overwrite/remove data as a side effect.
- Diff adds `--` to every built-in decompression command argument list.
- Comments discuss why `--` is added per tool and alternative `./` path prefixing.

## True Risks

- severity: `high`
- expected reviewer concern: command invocation safety for filenames beginning with `-`, compatibility for every decompression tool.
- risk mechanism: external process arguments can reinterpret filenames as flags, causing failure or side effects.

## Non-Risks

- Generic test-gap evidence is secondary if semantic command argument evidence is present.

## Expected Evidence

- useful: command argument/default array changes adding `--` for each decompressor.
- misleading: treating all repeated argument-array changes as independent high-risk items without grouping.
