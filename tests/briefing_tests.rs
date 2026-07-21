use openprose_lint::briefing::generate_briefing;
use std::path::Path;

#[test]
fn briefing_single_file() {
    let source = include_str!("../fixtures/briefing/single-file.md");
    let path = Path::new("fixtures/briefing/single-file.md");
    let briefing = generate_briefing(path, source);

    let expected = "\
<!-- openprose-lint briefing v2 -->
## test-discovery
kind: program | services: 2 | imports: 0

### contract
requires:
- topic: the subject to research
- depth: (optional, default \"shallow\") how deep to go
context: (none)
ensures:
- report: compiled findings on the topic
- sources: list of URLs consulted
errors:
- no-data: insufficient public information on the topic
environment: (none)

### services
researcher \u{2192} inline
compiler \u{2192} inline

### features
environment: no | use-imports: no | run-inputs: no | execution-block: yes

### diagnostics
0 errors, 0 warnings
";

    assert_eq!(briefing, expected);
}

#[test]
fn briefing_pure_contract() {
    let source = include_str!("../fixtures/briefing/pure-contract.md");
    let path = Path::new("fixtures/briefing/pure-contract.md");
    let briefing = generate_briefing(path, source);

    let expected = "\
<!-- openprose-lint briefing v2 -->
## status-check
kind: program | services: 2 | imports: 0

### contract
requires:
- runs_dir: (optional, default \".prose/runs/\") path to the runs directory
context: (none)
ensures:
- summary: summary of recent runs
errors:
- no-runs: no run data found
environment: (none)

### services
scanner \u{2192} vm-managed
summarizer \u{2192} vm-managed

### features
environment: no | use-imports: no | run-inputs: no | execution-block: no

### diagnostics
0 errors, 0 warnings
";

    assert_eq!(briefing, expected);
}

#[test]
fn briefing_with_imports() {
    let source = include_str!("../fixtures/briefing/with-imports.md");
    let path = Path::new("fixtures/briefing/with-imports.md");
    let briefing = generate_briefing(path, source);

    let expected = "\
<!-- openprose-lint briefing v2 -->
## daily-delivery
kind: program | services: 3 | imports: 2

### contract
requires:
- target: the subject to research
- gate_level: (optional, default \"external\") review level
context: (none)
ensures:
- report: the research output
- delivered: confirmation of Slack delivery
errors: (none)
environment:
- SLACK_WEBHOOK_URL
- SLACK_BOT_TOKEN

### services
research \u{2192} inline
human-gate \u{2192} use: std/delivery/human-gate
notifier \u{2192} inline

### features
environment: yes | use-imports: yes | run-inputs: no | execution-block: yes

### diagnostics
0 errors, 0 warnings
";

    assert_eq!(briefing, expected);
}

#[test]
fn briefing_includes_candidate_context_section() {
    let source = "\
---
name: context-briefing
kind: responsibility
version: 0.15.0
id: 067NC4KG01RG50R40M30E20918
---

### Context

- brief: declared caller context.

### Maintains

- answer: answer grounded in declared context.
";
    let briefing = generate_briefing(Path::new("context-briefing.prose.md"), source);

    assert!(
        briefing.contains("context:\n- brief: declared caller context."),
        "briefing should preserve candidate context section: {briefing}"
    );
    assert!(
        briefing.contains("maintains:\n- answer: answer grounded in declared context."),
        "briefing should preserve responsibility output contract: {briefing}"
    );
    assert!(
        !briefing.contains("ensures: (none)"),
        "responsibility briefing should not imply a missing program-style ensures section: {briefing}"
    );
    assert!(
        briefing.contains("0 errors, 0 warnings"),
        "context section should not cause missing-maintains warnings: {briefing}"
    );
}

#[test]
fn briefing_includes_function_parameters_and_returns() {
    let source = "\
---
name: context-function-briefing
kind: function
version: 0.15.0
---

### Context

- document: caller-provided document available to this function.

### Parameters

- document: text to summarize.

### Returns

- summary: concise grounded summary.
";
    let briefing = generate_briefing(Path::new("context-function-briefing.prose.md"), source);

    assert!(
        briefing
            .contains("context:\n- document: caller-provided document available to this function."),
        "briefing should preserve function context section: {briefing}"
    );
    assert!(
        briefing.contains("parameters:\n- document: text to summarize."),
        "briefing should preserve function parameters: {briefing}"
    );
    assert!(
        briefing.contains("returns:\n- summary: concise grounded summary."),
        "briefing should preserve function returns: {briefing}"
    );
    assert!(
        !briefing.contains("ensures: (none)"),
        "function briefing should not imply a missing program-style ensures section: {briefing}"
    );
}
