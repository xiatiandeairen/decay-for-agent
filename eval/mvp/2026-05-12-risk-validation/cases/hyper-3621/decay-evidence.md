# Decay evidence: hyper-3621

Source isolation: this analysis used only `decay.stdout`.

## Evidence

### evidence-1

- Category: stability
- Kind: panic_like_added
- Severity: high
- File/function: `src/server/conn/http1.rs`, line 504
- Raw message: `category=stability kind=panic_like_added subject=line 504 source=stability`
- Why decay reported it: a new panic-like pattern was detected.
- Actionability: review whether the line introduces `panic`/`unwrap`/`expect`, and whether a guard or invariant exists.

### evidence-2

- Category: quality
- Kind: blast_radius
- Severity: medium
- File/function: `src/client/conn/http2.rs`
- Raw message: `category=quality kind=blast_radius subject=multiple crates or domains source=quality`
- Why decay reported it: the change touches multiple crates/domains.
- Actionability: check whether the change is a coherent unit or should be split.

### evidence-3

- Category: quality
- Kind: mixed_concerns
- Severity: medium
- File/function: `src/client/conn/http2.rs`
- Raw message: `category=quality kind=mixed_concerns subject=multiple production domains source=quality`
- Why decay reported it: the production diff appears to mix multiple concerns.
- Actionability: check whether each concern has its own rationale and validation.

## Case assessment

- Evidence overload: low
- Signal state: non-null
- Top signals: `panic_like_added`, `blast_radius`, `mixed_concerns`

