# M3.1 Agent Review Rubric

## Purpose

Use independent subagent reviews to decide whether real PR candidates are strong enough for frozen eval. The goal is not to prove a candidate is valuable, but to reduce single-reviewer bias.

## Roles

- Ground Truth Agent: judges whether the real-world risk is clear from PR, issue, review, tests, and follow-up evidence. It must not use decay output.
- Risk Signal Agent: judges whether the case maps to transferable risk signals rather than project-specific trivia.
- Skeptic Agent: searches for evidence gaps, noise, overfit risk, and reasons not to freeze the case.

## Scores

Ground Truth Agent:

- `real_risk_clarity`: 0-5
- `evidence_strength`: 0-5
- `production_diff_relevance`: 0-5
- `follow_up_support`: 0-5
- `test_signal_quality`: 0-5

Risk Signal Agent:

- `transferability`: 0-5
- `signal_fit`: 0-5
- `signal_specificity`: 0-5
- `production_diff_relevance`: 0-5
- `noise_risk`: 0-5, higher is worse

Skeptic Agent:

- `noise_risk`: 0-5, higher is worse
- `evidence_gap`: 0-5, higher is worse
- `overfit_risk`: 0-5, higher is worse
- `project_specificity`: 0-5, higher is worse
- `missing_context_risk`: 0-5, higher is worse

## Aggregation Gates

`freeze_ready` requires all of:

- Ground Truth decision is not `reject`.
- Risk Signal decision is not `reject`.
- Skeptic decision is not `reject`.
- Ground Truth `real_risk_clarity >= 4`.
- Ground Truth `evidence_strength >= 4`.
- Ground Truth `production_diff_relevance >= 4`.
- Risk Signal `transferability >= 3`.
- Risk Signal `signal_fit >= 4`.
- Risk Signal `production_diff_relevance >= 4`.
- Risk Signal `noise_risk <= 2`.
- Skeptic `noise_risk <= 2`.
- Skeptic `evidence_gap <= 2`.
- Skeptic `missing_context_risk <= 2`.

`reject` applies when:

- Any agent gives high-confidence `reject`.
- The case has no production diff relevance.
- The case is mainly dependency/release-note/WIP/test-only noise.
- The risk is real but too project-specific to validate transferable signal behavior.

All other cases are `needs_more_context`.

## Important Limit

Agent agreement is not proof of product value. It only decides whether a case is strong enough to enter frozen eval. Product value still requires running the semantic flow and comparing against fact-only output.
