//! G181 (M14, ADR 0094): the lowerable body of a function, when its whole body is one tail
//! expression inside a bounded, pure, call-closed subset (`BodyNodeKind`).
//!
//! Syntax only, like the rest of this extractor: a path is recorded as written and never resolved
//! here (whether `Self::Local` names a variant is the construction lifter's question, answered
//! against the module it builds). Anything outside the subset -- a statement, a call, a macro, an
//! operator other than `==`/`!=`, parentheses, a guard, a binding pattern, an attribute -- makes the
//! whole body unrecorded. There is no partial body.

use atlas_core::{BodyNode, BodyNodeKind};

fn node(kind: BodyNodeKind, text: Option<String>, children: Vec<BodyNode>) -> BodyNode {
    BodyNode {
        kind,
        text,
        children,
    }
}

/// `a::b::C` for a path without a leading `::`, a qualified self or generic arguments.
fn plain_path(qself: &Option<syn::QSelf>, path: &syn::Path) -> Option<String> {
    if qself.is_some() || path.leading_colon.is_some() {
        return None;
    }
    let mut segments = Vec::new();
    for segment in &path.segments {
        if !segment.arguments.is_none() {
            return None;
        }
        segments.push(segment.ident.to_string());
    }
    Some(segments.join("::"))
}

fn expression(expr: &syn::Expr) -> Option<BodyNode> {
    match expr {
        syn::Expr::Lit(lit) if lit.attrs.is_empty() => match &lit.lit {
            syn::Lit::Bool(value) => Some(node(
                BodyNodeKind::Literal,
                Some(value.value.to_string()),
                Vec::new(),
            )),
            _ => None,
        },
        syn::Expr::Path(path) if path.attrs.is_empty() => Some(node(
            BodyNodeKind::Path,
            Some(plain_path(&path.qself, &path.path)?),
            Vec::new(),
        )),
        syn::Expr::Binary(binary) if binary.attrs.is_empty() => {
            let op = match binary.op {
                syn::BinOp::Eq(_) => "==",
                syn::BinOp::Ne(_) => "!=",
                _ => return None,
            };
            Some(node(
                BodyNodeKind::Binary,
                Some(op.to_owned()),
                vec![expression(&binary.left)?, expression(&binary.right)?],
            ))
        }
        syn::Expr::Match(m) if m.attrs.is_empty() => {
            let mut children = vec![expression(&m.expr)?];
            for arm in &m.arms {
                if !arm.attrs.is_empty() {
                    return None;
                }
                let mut parts = patterns(&arm.pat)?;
                parts.push(expression(&arm.body)?);
                children.push(node(BodyNodeKind::Arm, None, parts));
            }
            Some(node(BodyNodeKind::Match, None, children))
        }
        _ => None,
    }
}

/// The alternatives of an arm's pattern: paths and `_` only (a binding is outside the subset).
fn patterns(pat: &syn::Pat) -> Option<Vec<BodyNode>> {
    match pat {
        syn::Pat::Or(or) if or.attrs.is_empty() && or.leading_vert.is_none() => {
            let mut out = Vec::new();
            for case in &or.cases {
                out.extend(patterns(case)?);
            }
            Some(out)
        }
        syn::Pat::Path(path) if path.attrs.is_empty() => Some(vec![node(
            BodyNodeKind::Path,
            Some(plain_path(&path.qself, &path.path)?),
            Vec::new(),
        )]),
        syn::Pat::Wild(wild) if wild.attrs.is_empty() => {
            Some(vec![node(BodyNodeKind::Wildcard, None, Vec::new())])
        }
        _ => None,
    }
}

/// The body as one lowerable expression, or `None` when any part of it is outside the subset.
pub(super) fn lowerable_body(block: &syn::Block) -> Option<BodyNode> {
    match block.stmts.as_slice() {
        [syn::Stmt::Expr(expr, None)] => expression(expr),
        _ => None,
    }
}
