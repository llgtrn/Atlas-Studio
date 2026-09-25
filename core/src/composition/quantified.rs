//! Census-quantified invariants (G130, NA-CONSTRAINT-NONGROUND, ADR 0050).
//!
//! An ADL `invariant` may quantify over census truth: `forall f: function in Core forbid effect
//! PROCESS_SPAWN` or `forall f: function in Core forbid call to Runtime`. The ADL compiler cannot
//! decide it; this module decides it over the composed world model with a three-valued verdict:
//! - `VIOLATED` when a definite counterexample exists (an OBSERVED or DERIVED effect site, a
//!   resolved call), each one named;
//! - `SATISFIED` only when nothing that could be a counterexample is left unexamined: the dimension
//!   is OBSERVED on every file of the subsystem, no guessed (INFERRED) site of the category exists,
//!   and every call site either resolves or cannot reach the target -- or no Cargo dependency path
//!   exists at all, which no call can cross;
//! - `UNKNOWN` otherwise, with the residual that kept it from being decided.
//!
//! `observed effect within` (G137, ADR 0055) is the declared/observed effect reconciliation: it
//! claims only that every definite effect site lies inside the declared envelope, so it is always
//! decided, and its basis names the INFERRED sites and unobserved files it does not cover.
//!
//! `forbid effect` is about direct effect sites of the subsystem's own functions; an effect
//! reached through a call into another subsystem is that subsystem's.

use super::WorldModel;
use crate::EpistemicStatus;
use crate::constraint::ConstraintVerdict;
use crate::language::adl::{
    AdlCompileReport, AdlDiagnostic, CensusForbidden, ConstraintCheck, ConstraintCheckDerivation,
    ConstraintCheckKind, ConstraintResult,
};
use std::collections::BTreeSet;

/// Counterexamples named in diagnostics before the rest are counted.
const NAMED_COUNTEREXAMPLES: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub verdict: ConstraintVerdict,
    pub counterexamples: Vec<String>,
    /// What the verdict rests on: the evidence for SATISFIED, the residual for UNKNOWN.
    pub basis: Vec<String>,
}

/// Languages that hold data or configuration, never a function: a file in one of them cannot hide
/// a counterexample. Every other file (Rust, an included Rust fragment, a script, an unknown
/// language) can, so its coverage counts.
const NON_CODE_LANGUAGES: &[&str] = &["toml", "json", "markdown", "css", "yaml"];

/// Files of `subsystem` that can hold a function and whose `dimension` is not OBSERVED.
fn unobserved_files(model: &WorldModel, subsystem: &str, dimension: &str) -> usize {
    model
        .components
        .iter()
        .filter(|c| c.subsystem.as_deref() == Some(subsystem))
        .filter(|c| {
            !c.language
                .as_deref()
                .is_some_and(|l| NON_CODE_LANGUAGES.contains(&l))
        })
        .filter(|c| c.coverage.get(dimension) != Some(&EpistemicStatus::Observed))
        .count()
}

/// Decide `forbidden` for every function of `subsystem` in `model`.
pub fn decide(model: &WorldModel, subsystem: &str, forbidden: &CensusForbidden) -> Decision {
    let unknown = |basis: String| Decision {
        verdict: ConstraintVerdict::Unknown,
        counterexamples: Vec::new(),
        basis: vec![basis],
    };
    if !model.subsystems.iter().any(|s| s.name == subsystem) {
        return unknown(format!(
            "no subsystem `{subsystem}` in the composed census (a declared entity with a \
             materialization path)"
        ));
    }
    let functions: Vec<&super::FunctionBehavior> = model
        .functions
        .iter()
        .filter(|f| f.subsystem.as_deref() == Some(subsystem))
        .collect();
    let label = |f: &super::FunctionBehavior| format!("{}@{}:{}", f.name, f.path, f.line);
    match forbidden {
        CensusForbidden::Effect { category } => {
            let mut counterexamples = Vec::new();
            let mut guessed = 0;
            for f in &functions {
                for site in f.effects.iter().filter(|s| &s.kind == category) {
                    match site.status {
                        EpistemicStatus::Observed | EpistemicStatus::Derived => counterexamples
                            .push(format!(
                                "{} performs {category} at line {}",
                                label(f),
                                site.line
                            )),
                        _ => guessed += 1,
                    }
                }
            }
            if !counterexamples.is_empty() {
                return Decision {
                    verdict: ConstraintVerdict::Violated,
                    counterexamples,
                    basis: Vec::new(),
                };
            }
            let mut residual = Vec::new();
            let files = unobserved_files(model, subsystem, "EFFECT");
            if files > 0 {
                residual.push(format!(
                    "EFFECT not OBSERVED on {files} files of {subsystem}"
                ));
            }
            if guessed > 0 {
                residual.push(format!(
                    "{guessed} INFERRED {category} sites in {subsystem} (spelling guesses, \
                     neither confirmed nor refuted)"
                ));
            }
            if residual.is_empty() {
                Decision {
                    verdict: ConstraintVerdict::Satisfied,
                    counterexamples: Vec::new(),
                    basis: vec![format!(
                        "EFFECT OBSERVED on every file of {subsystem}; no {category} site in its \
                         {} functions",
                        functions.len()
                    )],
                }
            } else {
                Decision {
                    verdict: ConstraintVerdict::Unknown,
                    counterexamples: Vec::new(),
                    basis: residual,
                }
            }
        }
        CensusForbidden::ObservedEffectOutside { allowed } => {
            let mut counterexamples = Vec::new();
            let mut definite = 0;
            let mut guessed = 0;
            for f in &functions {
                for site in &f.effects {
                    match site.status {
                        EpistemicStatus::Observed | EpistemicStatus::Derived => {
                            definite += 1;
                            if !allowed.contains(&site.kind) {
                                counterexamples.push(format!(
                                    "{} performs {} at line {}, outside the envelope",
                                    label(f),
                                    site.kind,
                                    site.line
                                ));
                            }
                        }
                        _ => guessed += 1,
                    }
                }
            }
            if !counterexamples.is_empty() {
                return Decision {
                    verdict: ConstraintVerdict::Violated,
                    counterexamples,
                    basis: Vec::new(),
                };
            }
            // Decided over the definite sites alone: what the claim does not cover is named, not
            // guessed.
            let envelope = if allowed.is_empty() {
                "none".to_owned()
            } else {
                allowed.join(", ")
            };
            let mut basis = vec![format!(
                "all {definite} definite (OBSERVED or DERIVED) effect sites of {subsystem}'s {} functions are within the envelope ({envelope})",
                functions.len()
            )];
            let files = unobserved_files(model, subsystem, "EFFECT");
            if guessed > 0 || files > 0 {
                basis.push(format!(
                    "not covered by the claim: {guessed} INFERRED effect sites and {files} files of {subsystem} where EFFECT is not OBSERVED"
                ));
            }
            Decision {
                verdict: ConstraintVerdict::Satisfied,
                counterexamples: Vec::new(),
                basis,
            }
        }
        CensusForbidden::CallTo { subsystem: target } => {
            if !model.subsystems.iter().any(|s| &s.name == target) {
                return unknown(format!("no subsystem `{target}` in the composed census"));
            }
            let by_id: std::collections::BTreeMap<&str, &super::FunctionBehavior> =
                model.functions.iter().map(|f| (f.id.as_str(), f)).collect();
            let counterexamples: Vec<String> = functions
                .iter()
                .flat_map(|f| {
                    f.calls.iter().filter_map(|c| {
                        let callee = by_id.get(c.as_str())?;
                        (callee.subsystem.as_deref() == Some(target.as_str()))
                            .then(|| format!("{} calls {}", label(f), label(callee)))
                    })
                })
                .collect();
            if !counterexamples.is_empty() {
                return Decision {
                    verdict: ConstraintVerdict::Violated,
                    counterexamples,
                    basis: Vec::new(),
                };
            }
            let dependency = format!("INV-DEPENDENCY:{subsystem}->{target}");
            if model
                .invariants
                .iter()
                .any(|i| i.id == dependency && i.status == EpistemicStatus::Observed)
            {
                return Decision {
                    verdict: ConstraintVerdict::Satisfied,
                    counterexamples: Vec::new(),
                    basis: vec![format!(
                        "no Cargo dependency path from {subsystem} to {target} ({dependency}, \
                         OBSERVED): no call site, resolved or not, can reach {target}"
                    )],
                };
            }
            let target_names: BTreeSet<&str> = model
                .functions
                .iter()
                .filter(|f| f.subsystem.as_deref() == Some(target.as_str()))
                .map(|f| f.name.as_str())
                .collect();
            let candidates: usize = functions
                .iter()
                .flat_map(|f| f.unresolved_callee_names.iter())
                .filter(|(name, _)| target_names.contains(name.as_str()))
                .map(|(_, n)| n)
                .sum();
            let unnamed: usize = functions.iter().map(|f| f.unnamed_unresolved_calls).sum();
            let files = unobserved_files(model, subsystem, "CALL");
            let mut residual = Vec::new();
            if files > 0 {
                residual.push(format!("CALL not OBSERVED on {files} files of {subsystem}"));
            }
            if candidates > 0 {
                residual.push(format!(
                    "{candidates} unresolved call sites in {subsystem} are spelled with the name \
                     of a function of {target}"
                ));
            }
            if unnamed > 0 {
                residual.push(format!(
                    "{unnamed} unresolved call sites in {subsystem} have no callee name (closures, \
                     function pointers) and may reach {target}"
                ));
            }
            if residual.is_empty() {
                Decision {
                    verdict: ConstraintVerdict::Satisfied,
                    counterexamples: Vec::new(),
                    basis: vec![format!(
                        "a Cargo path from {subsystem} to {target} exists, but CALL is OBSERVED on \
                         every file of {subsystem} and no call site of it resolves to or is \
                         spelled with a function of {target}"
                    )],
                }
            } else {
                Decision {
                    verdict: ConstraintVerdict::Unknown,
                    counterexamples: Vec::new(),
                    basis: residual,
                }
            }
        }
    }
}

/// Whether any declared constraint or invariant quantifies over the census.
pub fn has_census_checks(adl: &AdlCompileReport) -> bool {
    let declared = &adl.ir.declared;
    declared
        .constraints
        .iter()
        .chain(&declared.invariants)
        .flat_map(|c| &c.checks)
        .any(|c| matches!(c, ConstraintCheck::CensusForbid { .. }))
}

/// Replace every census-quantified result in `adl.constraint_results` with its decision over
/// `model`.
pub fn apply(model: &WorldModel, adl: &mut AdlCompileReport) {
    let declared = adl.ir.declared.clone();
    for constraint in declared.constraints.iter().chain(&declared.invariants) {
        let census_checks: Vec<(&String, &CensusForbidden)> = constraint
            .checks
            .iter()
            .filter_map(|c| match c {
                ConstraintCheck::CensusForbid {
                    subsystem,
                    forbidden,
                } => Some((subsystem, forbidden)),
                _ => None,
            })
            .collect();
        // Only a declaration whose checks are all census-quantified is decided here; the parser
        // never mixes them, and a mixed one keeps its UNKNOWN.
        if census_checks.is_empty() || census_checks.len() != constraint.checks.len() {
            continue;
        }
        let diagnostic = |code: &str, message: String| AdlDiagnostic {
            code: code.into(),
            severity: "error".into(),
            message,
            span: constraint.span.clone(),
        };
        let mut verdicts = Vec::new();
        let mut diagnostics = Vec::new();
        let mut derivation = Vec::new();
        for (subsystem, forbidden) in census_checks {
            let decision = decide(model, subsystem, forbidden);
            verdicts.push(decision.verdict);
            match decision.verdict {
                ConstraintVerdict::Violated => {
                    let total = decision.counterexamples.len();
                    for counterexample in
                        decision.counterexamples.iter().take(NAMED_COUNTEREXAMPLES)
                    {
                        diagnostics.push(diagnostic(
                            "ATLAS-E064",
                            format!(
                                "invariant `{}` counterexample: {counterexample}",
                                constraint.name
                            ),
                        ));
                    }
                    if total > NAMED_COUNTEREXAMPLES {
                        diagnostics.push(diagnostic(
                            "ATLAS-E064",
                            format!(
                                "invariant `{}`: {} more counterexamples",
                                constraint.name,
                                total - NAMED_COUNTEREXAMPLES
                            ),
                        ));
                    }
                }
                ConstraintVerdict::Unknown => diagnostics.push(diagnostic(
                    "ATLAS-E063",
                    format!(
                        "invariant `{}` is undecided over the census: {}",
                        constraint.name,
                        decision.basis.join("; ")
                    ),
                )),
                ConstraintVerdict::Satisfied => {}
            }
            let mut supporting_node_names = vec![subsystem.clone()];
            if let CensusForbidden::CallTo { subsystem } = forbidden {
                supporting_node_names.push(subsystem.clone());
            }
            derivation.push(ConstraintCheckDerivation {
                rule: ConstraintCheckKind::CensusQuantified,
                supporting_node_names,
                materialization_target: None,
                basis: decision.basis,
            });
        }
        let verdict = ConstraintVerdict::all(verdicts);
        let result = ConstraintResult {
            name: constraint.name.clone(),
            passed: verdict.admits(),
            verdict,
            diagnostics,
            derivation,
        };
        if let Some(existing) = adl
            .constraint_results
            .iter_mut()
            .find(|r| r.name == constraint.name)
        {
            *existing = result;
        }
    }
}
