## docs

- roadmap: docs/roadmap.md
- prd/function-complexity-detection: docs/requirements/function-complexity-detection/prd.md
- arch/decay: docs/arch/decay.md
- ops: docs/ops.md
- decision/v0.1.0-closeout: docs/decision/v0.1.0-closeout.md
- milestone/m1: docs/milestones/m1.md
- milestone/m2: docs/milestones/m2.md

## know

- when: entering this repository or updating project docs
  must: treat v0.1.0 as a functional PoC whose product value is still unproven — tests passing does not mean the commit-decision hypothesis is validated
  how: use docs/roadmap.md and docs/ops.md as the source of truth; record dogfood cases as fixed / ignored / noise before claiming product success
  until: dogfood records show at least one real fixed case and the roadmap updates v0.1.0 status

- when: adding, removing, or renaming complexity metrics
  must: update the central metric registry first and keep README, PRD, architecture, diff, doctor, store, and tests aligned
  how: active metrics live in src/metric/mod.rs; product commitments are documented in README.md, README.zh.md, docs/arch/decay.md, and docs/requirements/function-complexity-detection/prd.md
  until: metric storage moves to a key/value schema with generated docs

- when: using know in this project
  prefer: use /know learn for reusable rules and /know write for structured docs; reject low-entropy notes instead of growing CLAUDE.md
  how: project docs are intentionally limited to roadmap, PRD, architecture, ops, and decision docs in know-standard paths
  until: know write path rules change
