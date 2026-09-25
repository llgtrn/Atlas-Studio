//! Call site identity.
//!
//! R4.5 (`.atlas/contracts/SEMANTIC-FACTS.md#callfact`): "A call record contains caller, callsite,
//! candidate/resolved callee identities, dispatch kind and argument/result bindings when known.
//! Dynamic calls are explicit; failure to resolve never erases the call." The Rust extractor has no
//! rustc-backed name resolution (never will, per every prior wave's epistemic discipline), so it
//! can soundly observe WHERE a call syntactically occurs and WHO makes it, but not, in the general
//! case, WHOM it calls -- an unqualified path or a method-call receiver's type is not determinable
//! from `syn` alone. `dispatch`/`callees` are therefore always `Unresolved`/`[]` from that
//! extractor. Since G75 a second engine, native Rust name resolution of path calls
//! (`adapter::semantic::rust::resolve`, absorbed from rust-analyzer's `hir-def`; never rustc), observes
//! the same claims with `StaticResolved` and the callee's FunctionIdentity record -- the reason
//! these fields were never part of the identity.

use super::SemanticRecordId;
use super::place::PlaceRef;
use crate::identity::RepositoryId;
use crate::language::adl::SourceSpan;
use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

/// `.atlas/contracts/SEMANTIC-FACTS.md#callfact`'s four dispatch outcomes, verbatim.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CallDispatchKind {
    StaticResolved,
    DynamicResolvedSet,
    DynamicPartial,
    Unresolved,
}

impl CallDispatchKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::StaticResolved => "STATIC_RESOLVED",
            Self::DynamicResolvedSet => "DYNAMIC_RESOLVED_SET",
            Self::DynamicPartial => "DYNAMIC_PARTIAL",
            Self::Unresolved => "UNRESOLVED",
        }
    }
}

/// Identity of one call site, owned by exactly one function.
///
/// `dispatch`/`callees`/`arguments`/`result` are deliberately NOT part of `identity_key()`: they are
/// RESOLUTION FACTS about an already-uniquely-identified call site (identified by which function
/// makes it and at which source span), not part of the site's identity. Keeping them out of the
/// identity means `record_id` stays stable if a later wave resolves a call this extractor could
/// only mark `Unresolved` today -- re-extraction after a resolution improvement updates this
/// record's content without orphaning any existing reference to it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CallSiteIdentity {
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    /// The CALLER: the `FunctionIdentity` record_id of the function this call site is inside.
    pub function: SemanticRecordId,
    /// The call's anchor token: the callee's name (a method call's method identifier, a path
    /// call's last path segment), or the argument list's opening parenthesis when the callee is
    /// not a path. Each token belongs to exactly one call expression, so the calls of a chain
    /// (`a.b().c()`, which all START at `a`) stay distinct call sites (G74), and an independent
    /// engine that anchors references at the name (SCIP, LSIF) names the same claim.
    pub span: SourceSpan,
    pub dispatch: CallDispatchKind,
    /// Candidate/resolved callee `FunctionIdentity` record_ids. Empty whenever `dispatch ==
    /// Unresolved`; MAY be non-empty for `DynamicPartial`/`DynamicResolvedSet` (a candidate set)
    /// or hold exactly one entry for `StaticResolved`.
    pub callees: Vec<SemanticRecordId>,
    /// R4.12: one `PlaceRef` per syntactic argument position, in source order, referencing the
    /// DATA_FLOW `Use` this argument expression corresponds to when the extractor can prove it --
    /// `PlaceRef::Resolved { dimension: DataFlow, record_id }` for a simple single-identifier
    /// argument (`helper(x)`), `PlaceRef::Unresolved` for anything requiring deeper analysis
    /// (`helper(w.get())`, a literal, a nested call, ...). Never derived from argument spelling
    /// alone -- the `record_id` is computed via the SAME `ValueIdentity::identity_key()` formula
    /// DATA_FLOW's own walker uses, so it names a record DATA_FLOW's own pass produces, not an
    /// independently invented one (see the extractor's `build_calls` doc comment for the
    /// convergence proof and the requested-dimension gating this relies on).
    pub arguments: Vec<PlaceRef>,
    /// R4.12: a `PlaceRef` to the DATA_FLOW `Definition`/`Store` this call's return value directly
    /// becomes, when the extractor can prove it -- `PlaceRef::Resolved { dimension: DataFlow,
    /// record_id }` when this call expression is EXACTLY the direct initializer of a simple
    /// (non-destructured) `let` binding or the direct right-hand side of a simple assignment
    /// (`let y = helper(x);`, `y = helper(x);`), `PlaceRef::Unresolved` otherwise -- including when
    /// the call's value is merely a SUBEXPRESSION of a larger one (`let y = helper(x) + 1;` has no
    /// single value this call's result "becomes"). Computed the same way `arguments` is: reusing
    /// `ValueIdentity::identity_key()` itself against the binding's own identifier/span, never a
    /// hand-duplicated formula.
    pub result: PlaceRef,
    /// G123 (GAP-UNRESOLVED-CALLEE): the callee as written -- `foo`, `Type::method`, `.method`, or
    /// a token summary of a non-path callee expression. OBSERVED syntax, never a resolution, and
    /// not part of `identity_key()`. `None` for engines that do not read call syntax.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callee_spelling: Option<String>,
}

/// The name an unresolved call site's spelling calls by: `.method` -> `method`, `a::b::<T>` ->
/// `b`, `foo` -> `foo`. `None` when the callee is not a path or a method (a closure, a function
/// pointer expression, a parenthesized call): such a site may reach any function.
pub fn callee_name(spelling: &str) -> Option<String> {
    let spelling = spelling.trim();
    let path = spelling.strip_prefix('.').unwrap_or(spelling);
    // The path with its generic argument groups (`::<u8>`, `<T as Trait>`) removed.
    let mut depth = 0usize;
    let mut plain = String::new();
    for c in path.chars() {
        match c {
            '<' => depth += 1,
            '>' => depth = depth.checked_sub(1)?,
            c if depth == 0 => plain.push(c),
            _ => {}
        }
    }
    if depth != 0
        || !plain
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
    {
        return None;
    }
    let last = plain.trim_end_matches(':').rsplit("::").next()?;
    if last.is_empty() || last.starts_with(|c: char| c.is_ascii_digit()) || last.contains(':') {
        return None;
    }
    Some(last.to_owned())
}

impl CallSiteIdentity {
    /// Deterministic, order-independent encoding of this call site's identity fields.
    pub fn identity_key(&self) -> String {
        format!(
            "{}|{}:{}|{}|{}:{}:{}",
            self.repository.as_str(),
            self.revision.kind,
            self.revision.value,
            self.function.as_str(),
            self.span.path,
            self.span.line,
            self.span.column,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::SemanticDimension;
    use super::*;

    #[test]
    fn callee_names_come_from_paths_and_methods_only() {
        assert_eq!(callee_name(".bump").as_deref(), Some("bump"));
        assert_eq!(callee_name("helper").as_deref(), Some("helper"));
        assert_eq!(
            callee_name("crate::store::Store::new").as_deref(),
            Some("new")
        );
        assert_eq!(
            callee_name("Vec::<u8>::with_capacity").as_deref(),
            Some("with_capacity")
        );
        assert_eq!(callee_name("parse::<u32>").as_deref(), Some("parse"));
        assert_eq!(callee_name("<T as Trait>::m").as_deref(), Some("m"));
        assert_eq!(callee_name("(get_fn())"), None, "a non-path callee");
        assert_eq!(
            callee_name("self . handler"),
            None,
            "a field holding a function"
        );
        assert_eq!(callee_name("closures [0]"), None);
        assert_eq!(callee_name(""), None);
    }

    fn base() -> CallSiteIdentity {
        CallSiteIdentity {
            repository: RepositoryId::new("atlas-studio"),
            revision: RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            function: SemanticRecordId::new(SemanticDimension::FunctionIdentity, "fn-key"),
            span: SourceSpan {
                path: "core/src/lib.rs".into(),
                line: 10,
                column: 5,
            },
            dispatch: CallDispatchKind::Unresolved,
            callees: Vec::new(),
            arguments: Vec::new(),
            result: PlaceRef::Unresolved,
            callee_spelling: None,
        }
    }

    #[test]
    fn identity_key_distinguishes_span() {
        let other = CallSiteIdentity {
            span: SourceSpan {
                line: 11,
                ..base().span
            },
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn identity_key_distinguishes_owning_function() {
        let other = CallSiteIdentity {
            function: SemanticRecordId::new(SemanticDimension::FunctionIdentity, "other-fn-key"),
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    // --- R4.5: dispatch/callees are content, not identity ---------------------------------------

    #[test]
    fn identity_key_is_unaffected_by_dispatch_and_callees() {
        // A later resolution improvement (Unresolved -> StaticResolved) must not orphan an
        // existing reference to this call site's record_id.
        let resolved = CallSiteIdentity {
            dispatch: CallDispatchKind::StaticResolved,
            callees: vec![SemanticRecordId::new(
                SemanticDimension::FunctionIdentity,
                "callee-key",
            )],
            ..base()
        };
        assert_eq!(base().identity_key(), resolved.identity_key());
    }

    // --- R4.12: arguments are content, not identity -----------------------------------------------

    #[test]
    fn identity_key_is_unaffected_by_arguments() {
        let with_arguments = CallSiteIdentity {
            arguments: vec![PlaceRef::Resolved {
                dimension: SemanticDimension::DataFlow,
                record_id: SemanticRecordId::new(SemanticDimension::DataFlow, "value-key"),
            }],
            ..base()
        };
        assert_eq!(base().identity_key(), with_arguments.identity_key());
    }

    #[test]
    fn identity_key_is_unaffected_by_result() {
        let with_result = CallSiteIdentity {
            result: PlaceRef::Resolved {
                dimension: SemanticDimension::DataFlow,
                record_id: SemanticRecordId::new(SemanticDimension::DataFlow, "definition-key"),
            },
            ..base()
        };
        assert_eq!(base().identity_key(), with_result.identity_key());
    }

    #[test]
    fn dispatch_kind_as_str_matches_the_contract_vocabulary() {
        assert_eq!(CallDispatchKind::StaticResolved.as_str(), "STATIC_RESOLVED");
        assert_eq!(
            CallDispatchKind::DynamicResolvedSet.as_str(),
            "DYNAMIC_RESOLVED_SET"
        );
        assert_eq!(CallDispatchKind::DynamicPartial.as_str(), "DYNAMIC_PARTIAL");
        assert_eq!(CallDispatchKind::Unresolved.as_str(), "UNRESOLVED");
    }
}
