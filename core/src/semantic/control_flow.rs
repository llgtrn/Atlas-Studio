//! Control-flow block identity.
//!
//! R4.6 (`.atlas/contracts/SEMANTIC-FACTS.md#controlflowfact`): "Represents blocks, terminators,
//! successors, exceptional/unwind edges and entry/exit relationships. CFG identity MUST be stable
//! for identical pinned input and MUST NOT depend on traversal/hash iteration order."
//!
//! Unlike R4.5's CALL dimension (where callee resolution genuinely requires name/type information
//! this extractor doesn't have and never will), a function's control-flow STRUCTURE -- which branch
//! executes when, where a loop repeats to, where a `return`/`break` actually goes -- is fully
//! determined by Rust's own language syntax and semantics, not by type inference or import
//! resolution. There is therefore no equivalent epistemic reason to leave those CFG edges
//! unresolved by default: `adapter`'s extractor computes real successor edges wherever source
//! syntax structurally determines them, and uses `ControlFlowEdgeKind::Unresolved` only for
//! genuinely ambiguous cases (e.g. a labeled `break`/`continue` whose label doesn't match any
//! enclosing construct this extractor tracked -- a real possibility in
//! syntactically-valid-but-semantically-odd input, since this extractor performs no name
//! resolution).
//!
//! A block whose only path out is a textual panic-like macro invocation (`Panic` edge kind, below)
//! is the one exception: unlike `return`/`break`, whether the invoked macro actually diverges is
//! NOT fully determined by syntax alone -- a local `macro_rules!` redefinition of `panic`/
//! `unreachable`/`todo`/`unimplemented` can make it do anything, and this extractor has no
//! macro/name resolution to rule that out (the same limitation R4.8's EFFECT already documents for
//! the identical evidence). Because the block's own successor set depends on this classification --
//! if the macro doesn't really diverge, the block's true successor is whatever syntactically
//! follows, not "leaves the function" -- the whole block's `SemanticRecordHeader::status` is
//! `EpistemicStatus::Inferred`, not `Observed`, whenever it terminates this way. Every other block
//! shape stays `Observed`.
//!
//! Scope this wave: only statement-level control flow is split into blocks (an `if`/`match`/
//! `loop`/`while`/`for`/bare-block used directly AS a statement or the tail of a statement list).
//! A construct nested inside a larger expression -- e.g. `let x = if c { 1 } else { 2 };` -- is not
//! given its own CFG blocks; its branches are walked for CALL purposes (R4.5) but contribute no
//! separate ControlFlow block/edge structure. Closures likewise get no CFG of their own (they have
//! no `FunctionIdentity` in Atlas's model to attribute one to). Both are real, documented gaps, not
//! silent omissions -- true expression-level CFG splitting is deferred to a future wave.
//!
//! A `let PAT = EXPR else { diverge }` statement is also split into its own decision point, one
//! more real branch shape alongside `if`/`match`/loops -- found missing, then closed, by direct
//! adversarial testing this session (see `adapter::semantic::rust::cfg`'s own module doc comment
//! and `ControlFlowBlockKind::LetElseDiverge` below).

use super::SemanticRecordId;
use crate::identity::RepositoryId;
use crate::temporal::RevisionRef;
use serde::{Deserialize, Serialize};

/// The structural role a block plays relative to its parent construct. Never part of
/// `identity_key()` -- see the module doc comment on `ControlFlowBlockIdentity`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ControlFlowBlockKind {
    /// The function/method's own outermost body block -- the CFG's entry block.
    FunctionEntry,
    /// The `{ .. }` taken when an `if` condition is true.
    IfThen,
    /// The `{ .. }` taken when an `if` condition is false (an explicit `else` block, never a
    /// synthesized empty block for a bare `if` with no `else`).
    IfElse,
    /// A bare `loop { .. }` body.
    LoopBody,
    /// A `while cond { .. }` body.
    WhileBody,
    /// A `for pat in iter { .. }` body.
    ForLoopBody,
    /// One `match` arm's body.
    MatchArm,
    /// A `{ .. }` used as an expression (e.g. `let x = { .. };`) or a bare statement block, not
    /// introduced by any of the constructs above. Also covers `unsafe { .. }`: `unsafe` grants
    /// permission for certain operations, it is not itself a distinct control-flow shape, so it
    /// shares this kind rather than a dedicated one.
    NestedBlockExpr,
    /// The statements following a branch/loop within the same enclosing statement list -- the
    /// "join" segment every non-diverging branch converges back into. Not backed by its own
    /// `syn` node distinct from its parent's statement list; it exists because that list had to
    /// be split at the branch/loop boundary.
    Continuation,
    /// A `let PAT = EXPR else { .. }` statement's diverge arm -- taken when the pattern fails to
    /// match. Must never complete normally in valid Rust (the diverge block's type is `!`), but
    /// this extractor still lowers its own statements like any other block rather than assuming
    /// that guarantee holds for untrusted/malformed input.
    LetElseDiverge,
}

impl ControlFlowBlockKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::FunctionEntry => "FUNCTION_ENTRY",
            Self::IfThen => "IF_THEN",
            Self::IfElse => "IF_ELSE",
            Self::LoopBody => "LOOP_BODY",
            Self::WhileBody => "WHILE_BODY",
            Self::ForLoopBody => "FOR_LOOP_BODY",
            Self::MatchArm => "MATCH_ARM",
            Self::NestedBlockExpr => "NESTED_BLOCK_EXPR",
            Self::Continuation => "CONTINUATION",
            Self::LetElseDiverge => "LET_ELSE_DIVERGE",
        }
    }
}

/// How control leaves a block, and where it provably goes. See
/// `.atlas/contracts/SEMANTIC-FACTS.md#controlflowfact`'s "terminators, successors,
/// exceptional/unwind edges".
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ControlFlowEdgeKind {
    /// Sequential continuation into another modeled block.
    Fallthrough,
    /// One arm of a conditional decision point (an `if`/`else`/`match` arm entry).
    Branch,
    /// Control returns to a loop body's own entry -- either the body completed normally
    /// (implicit repeat) or an explicit `continue` was taken. Both share this kind because both
    /// have the identical target (the loop body's entry block) and effect.
    LoopRepeat,
    /// Control leaves the function normally (an explicit `return`, the "break" outcome of a `?`
    /// statement propagating its value out of the enclosing function, or a block's implicit
    /// completion at function-body top level). Always `target: None`. All three share this kind
    /// because all three share the identical target (function exit, no further modeled block)
    /// and effect -- the same principle `LoopRepeat` already applies to its own two causes.
    Return,
    /// An explicit `break` (labeled or not), landing wherever the matched loop's own
    /// continuation leads. `target` is `Some` when that continuation is a concrete modeled
    /// block, `None` when it resolves to `Return` or another unresolved case further out.
    Break,
    /// A `panic!`/`unreachable!`/`todo!`/`unimplemented!`-spelled macro invocation -- textual
    /// evidence of abnormal termination, not proof of it (the macro name could be locally
    /// shadowed; see this module's doc comment). Always `target: None`. The owning block's
    /// `SemanticRecordHeader::status` is `Inferred`, never `Observed`, whenever a block's only
    /// successor is this edge kind.
    Panic,
    /// Control demonstrably leaves this block, but this extractor cannot determine where --
    /// e.g. a labeled `break`/`continue` whose label matched no loop this extractor tracked
    /// while walking outward from the block. Never silently dropped; always explicit (contract:
    /// "explicit unresolved control constructs").
    Unresolved,
}

impl ControlFlowEdgeKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Fallthrough => "FALLTHROUGH",
            Self::Branch => "BRANCH",
            Self::LoopRepeat => "LOOP_REPEAT",
            Self::Return => "RETURN",
            Self::Break => "BREAK",
            Self::Panic => "PANIC",
            Self::Unresolved => "UNRESOLVED",
        }
    }
}

/// One outgoing control-flow edge from a block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ControlFlowEdge {
    pub kind: ControlFlowEdgeKind,
    /// The destination block's record_id, when the destination is a block this extractor
    /// modeled. `None` for `Return`/`Panic`/`Unresolved` (all leave the modeled block set by
    /// definition), and for a `Break` whose own resolved continuation is itself `Return` or
    /// `Unresolved` further out.
    pub target: Option<SemanticRecordId>,
}

/// Identity of one CFG block, stable for the pinned semantic input (see
/// `.atlas/contracts/SEMANTIC-FACTS.md#controlflowfact`).
///
/// `kind`/`is_entry`/`successors` are deliberately NOT part of `identity_key()`, for the same
/// reason R4.5's `CallSiteIdentity` excludes `dispatch`/`callees`: `block_index` (combined with
/// `repository`/`revision`/`function`) already uniquely identifies which block this is within its
/// function, given the extractor's deterministic traversal order. `kind`/`successors` are CONTENT
/// describing that already-identified block, not additional identity-disambiguating fields -- so
/// `record_id` stays stable if a later wave computes a different (e.g. more precise) successor set
/// for the same block, without orphaning any existing reference to it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ControlFlowBlockIdentity {
    pub repository: RepositoryId,
    pub revision: RevisionRef,
    pub function: SemanticRecordId,
    pub block_index: usize,
    pub kind: ControlFlowBlockKind,
    pub is_entry: bool,
    pub successors: Vec<ControlFlowEdge>,
}

impl ControlFlowBlockIdentity {
    /// Deterministic, order-independent encoding of this block's identity fields.
    pub fn identity_key(&self) -> String {
        format!(
            "{}|{}:{}|{}|{}",
            self.repository.as_str(),
            self.revision.kind,
            self.revision.value,
            self.function.as_str(),
            self.block_index,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::SemanticDimension;
    use super::*;

    fn base() -> ControlFlowBlockIdentity {
        ControlFlowBlockIdentity {
            repository: RepositoryId::new("atlas-studio"),
            revision: RevisionRef {
                kind: "git".into(),
                value: "abc123".into(),
            },
            function: SemanticRecordId::new(SemanticDimension::FunctionIdentity, "fn-key"),
            block_index: 0,
            kind: ControlFlowBlockKind::FunctionEntry,
            is_entry: true,
            successors: Vec::new(),
        }
    }

    #[test]
    fn identity_key_distinguishes_block_index() {
        let other = ControlFlowBlockIdentity {
            block_index: 1,
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn identity_key_distinguishes_owning_function() {
        let other = ControlFlowBlockIdentity {
            function: SemanticRecordId::new(SemanticDimension::FunctionIdentity, "other-fn-key"),
            ..base()
        };
        assert_ne!(base().identity_key(), other.identity_key());
    }

    // --- R4.6: kind/is_entry/successors are content, not identity -----------------------------

    #[test]
    fn identity_key_is_unaffected_by_kind_is_entry_and_successors() {
        let other = ControlFlowBlockIdentity {
            kind: ControlFlowBlockKind::LoopBody,
            is_entry: false,
            successors: vec![ControlFlowEdge {
                kind: ControlFlowEdgeKind::Fallthrough,
                target: Some(SemanticRecordId::new(SemanticDimension::ControlFlow, "x")),
            }],
            ..base()
        };
        assert_eq!(base().identity_key(), other.identity_key());
    }

    #[test]
    fn block_kind_as_str_matches_the_screaming_snake_vocabulary() {
        assert_eq!(
            ControlFlowBlockKind::FunctionEntry.as_str(),
            "FUNCTION_ENTRY"
        );
        assert_eq!(ControlFlowBlockKind::IfThen.as_str(), "IF_THEN");
        assert_eq!(ControlFlowBlockKind::IfElse.as_str(), "IF_ELSE");
        assert_eq!(ControlFlowBlockKind::LoopBody.as_str(), "LOOP_BODY");
        assert_eq!(ControlFlowBlockKind::WhileBody.as_str(), "WHILE_BODY");
        assert_eq!(ControlFlowBlockKind::ForLoopBody.as_str(), "FOR_LOOP_BODY");
        assert_eq!(ControlFlowBlockKind::MatchArm.as_str(), "MATCH_ARM");
        assert_eq!(
            ControlFlowBlockKind::NestedBlockExpr.as_str(),
            "NESTED_BLOCK_EXPR"
        );
        assert_eq!(ControlFlowBlockKind::Continuation.as_str(), "CONTINUATION");
    }

    #[test]
    fn edge_kind_as_str_matches_the_contract_vocabulary() {
        assert_eq!(ControlFlowEdgeKind::Fallthrough.as_str(), "FALLTHROUGH");
        assert_eq!(ControlFlowEdgeKind::Branch.as_str(), "BRANCH");
        assert_eq!(ControlFlowEdgeKind::LoopRepeat.as_str(), "LOOP_REPEAT");
        assert_eq!(ControlFlowEdgeKind::Return.as_str(), "RETURN");
        assert_eq!(ControlFlowEdgeKind::Break.as_str(), "BREAK");
        assert_eq!(ControlFlowEdgeKind::Panic.as_str(), "PANIC");
        assert_eq!(ControlFlowEdgeKind::Unresolved.as_str(), "UNRESOLVED");
    }
}
