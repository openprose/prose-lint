use std::collections::HashSet;

use openprose_lint::known_contract_sections;

#[test]
fn known_contract_sections_include_canonical_h3_vocabulary() {
    let sections = known_contract_sections();

    for canonical in [
        "description",
        "goal",
        "requires",
        "maintains",
        "parameters",
        "returns",
        "ensures",
        "errors",
        "invariants",
        "strategies",
        "environment",
        "fixtures",
        "expects",
        "expects not",
    ] {
        assert!(
            sections.contains(&canonical),
            "missing canonical contract section: {canonical}"
        );
    }
}

#[test]
fn known_contract_sections_are_unique() {
    let sections = known_contract_sections();
    let unique: HashSet<_> = sections.iter().copied().collect();

    assert_eq!(
        unique.len(),
        sections.len(),
        "the authoritative contract-section vocabulary contains duplicates"
    );
}
