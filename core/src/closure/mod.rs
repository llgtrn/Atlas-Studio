//! Semi-naive, delta-driven fixed-point evaluation (R5 capability "recursive/fixed-point semantic
//! derivation"; `.atlas/decisions/0004-semi-naive-dependency-closure.md`).
//!
//! Natively re-derived from datafrog's `Variable`/`Iteration` mechanism
//! (`rust-lang/datafrog@edd7cfc`, `src/variable.rs` `VariableTrait::changed`,
//! `src/iteration.rs` `Iteration::changed`), not copied or depended on. A relation holds three
//! disjoint tuple sets -- `stable` (already processed), `recent` (derived last round, not yet
//! processed) and `to_add` (derived this round) -- and each round only the `recent` delta is fed
//! to the derivation step, so no tuple is ever expanded twice. That is what makes the loop both
//! terminate on cyclic input (a re-derived tuple is deduplicated against `stable` and never becomes
//! `recent` again) and expand every tuple exactly once, however many derivation paths reach it.
//!
//! Every run also produces a `FixedPointRecord` -- iteration count, whether convergence was
//! reached, and delta remaining at exit -- so a closure claim can be checked rather than asserted:
//! `.atlas/contracts/DEPENDENCY-CENSUS.md#closure` requires that "no new dependency nodes appear on
//! another closure iteration", and `.atlas/contracts/CENSUS-CERTIFICATE.md` records "whether
//! dependency closure reached fixed point". (That contract's separate census-wide reconciliation
//! fixed point is a different, not-yet-implemented loop; this record does not claim to be it.)
//!
//! Deliberately NOT absorbed (recorded in the ADR): datafrog's leapjoin/treefrog multi-way join,
//! antijoin, and its geometric batch-merging storage layout -- none has an Atlas caller yet.
//! `BTreeSet` gives the same deduplication semantics deterministically, in stable order.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The fixed-point accounting `.atlas/contracts/CENSUS-CERTIFICATE.md` requires of any closure
/// claim: how many delta rounds ran, whether the loop ended because no new tuple appeared
/// (`converged`), and how many unprocessed tuples remained when it stopped (`0` iff converged).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct FixedPointRecord {
    pub iterations: usize,
    pub converged: bool,
    pub delta_remaining: usize,
}

/// One monotone relation under semi-naive evaluation: datafrog's `Variable`, minus its
/// shared-ownership plumbing and batch layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeltaRelation<T: Ord> {
    stable: BTreeSet<T>,
    recent: BTreeSet<T>,
    to_add: BTreeSet<T>,
}

impl<T: Ord> Default for DeltaRelation<T> {
    fn default() -> Self {
        Self {
            stable: BTreeSet::new(),
            recent: BTreeSet::new(),
            to_add: BTreeSet::new(),
        }
    }
}

impl<T: Ord> DeltaRelation<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue a tuple for the next round. Not visible in `recent` until `changed` is called.
    pub fn insert(&mut self, tuple: T) {
        self.to_add.insert(tuple);
    }

    pub fn recent(&self) -> &BTreeSet<T> {
        &self.recent
    }

    pub fn stable(&self) -> &BTreeSet<T> {
        &self.stable
    }

    pub fn is_stable(&self) -> bool {
        self.recent.is_empty() && self.to_add.is_empty()
    }

    /// Advance one round: fold `recent` into `stable`, then promote every queued tuple that is not
    /// already `stable` into `recent`. Returns whether any genuinely new tuple appeared -- the
    /// round-over-round delta whose emptiness is the fixed-point condition.
    pub fn changed(&mut self) -> bool {
        self.stable.append(&mut self.recent);
        let queued = std::mem::take(&mut self.to_add);
        self.recent = queued
            .into_iter()
            .filter(|tuple| !self.stable.contains(tuple))
            .collect();
        !self.recent.is_empty()
    }

    /// Every tuple ever derived. Includes any unprocessed `recent`/`to_add` tuples when the caller
    /// stopped early, so a non-converged result is still a sound (if incomplete) under-approximation.
    pub fn into_all(mut self) -> BTreeSet<T> {
        self.stable.append(&mut self.recent);
        self.stable.append(&mut self.to_add);
        self.stable
    }
}

/// Run `derive` to a fixed point from `seed`, feeding it only each round's new tuples.
///
/// `derive(delta, emit)` must be monotone: it may only emit tuples implied by the tuples in `delta`
/// (joined against whatever static input it closes over). `max_rounds` is a defensive bound, not an
/// expected exit: over a finite tuple domain a monotone loop always converges, since every round
/// that continues added at least one tuple not already stable. If the bound is hit anyway, the
/// result is returned with `converged: false` and the unprocessed delta counted, never silently
/// truncated.
pub fn semi_naive_fixed_point<T, F>(
    seed: impl IntoIterator<Item = T>,
    max_rounds: usize,
    mut derive: F,
) -> (BTreeSet<T>, FixedPointRecord)
where
    T: Ord,
    F: FnMut(&BTreeSet<T>, &mut dyn FnMut(T)),
{
    let mut relation = DeltaRelation::new();
    for tuple in seed {
        relation.insert(tuple);
    }
    let mut iterations = 0;
    while relation.changed() {
        if iterations == max_rounds {
            let delta_remaining = relation.recent().len();
            return (
                relation.into_all(),
                FixedPointRecord {
                    iterations,
                    converged: false,
                    delta_remaining,
                },
            );
        }
        iterations += 1;
        let mut derived = Vec::new();
        derive(relation.recent(), &mut |tuple| derived.push(tuple));
        for tuple in derived {
            relation.insert(tuple);
        }
    }
    (
        relation.into_all(),
        FixedPointRecord {
            iterations,
            converged: true,
            delta_remaining: 0,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn successors(edges: &[(u32, u32)]) -> BTreeMap<u32, Vec<u32>> {
        let mut out: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for &(from, to) in edges {
            out.entry(from).or_default().push(to);
        }
        out
    }

    fn reach(roots: &[u32], edges: &[(u32, u32)]) -> (BTreeSet<u32>, FixedPointRecord) {
        let next = successors(edges);
        semi_naive_fixed_point(roots.iter().copied(), 1_000, |delta, emit| {
            for node in delta {
                for &to in next.get(node).into_iter().flatten() {
                    emit(to);
                }
            }
        })
    }

    #[test]
    fn changed_folds_recent_and_deduplicates_against_stable() {
        let mut relation = DeltaRelation::new();
        relation.insert(1);
        relation.insert(2);
        assert!(relation.changed());
        assert_eq!(relation.recent(), &BTreeSet::from([1, 2]));
        relation.insert(2);
        relation.insert(3);
        assert!(relation.changed());
        assert_eq!(relation.stable(), &BTreeSet::from([1, 2]));
        assert_eq!(
            relation.recent(),
            &BTreeSet::from([3]),
            "2 was already stable and must not be re-expanded"
        );
        assert!(!relation.changed());
        assert!(relation.is_stable());
        assert_eq!(relation.into_all(), BTreeSet::from([1, 2, 3]));
    }

    #[test]
    fn a_chain_converges_with_one_round_per_level() {
        let (all, record) = reach(&[0], &[(0, 1), (1, 2), (2, 3)]);
        assert_eq!(all, BTreeSet::from([0, 1, 2, 3]));
        assert_eq!(
            record,
            FixedPointRecord {
                iterations: 4,
                converged: true,
                delta_remaining: 0
            }
        );
    }

    #[test]
    fn a_cycle_terminates_instead_of_re_expanding_forever() {
        let (all, record) = reach(&[0], &[(0, 1), (1, 2), (2, 0), (2, 2)]);
        assert_eq!(all, BTreeSet::from([0, 1, 2]));
        assert!(record.converged);
        assert_eq!(record.iterations, 3);
    }

    #[test]
    fn a_node_reachable_at_two_depths_is_expanded_once() {
        // 3 is reachable at depth 1 (0->3) and again at depth 3 (0->1->2->3): unequal path
        // lengths, so the second derivation of 3 lands in a LATER round than the first -- the
        // cross-round case that only deduplication against `stable` prevents. (An equal-length
        // diamond would be deduplicated inside a single round's `to_add` set and prove nothing.)
        let mut expansions = BTreeMap::<u32, usize>::new();
        let next = successors(&[(0, 1), (0, 3), (1, 2), (2, 3), (3, 4)]);
        let (all, record) = semi_naive_fixed_point([0], 1_000, |delta, emit| {
            for node in delta {
                *expansions.entry(*node).or_default() += 1;
                for &to in next.get(node).into_iter().flatten() {
                    emit(to);
                }
            }
        });
        assert_eq!(all, BTreeSet::from([0, 1, 2, 3, 4]));
        assert!(record.converged);
        assert!(
            expansions.values().all(|count| *count == 1),
            "semi-naive evaluation must feed each tuple to derive exactly once: {expansions:?}"
        );
    }

    #[test]
    fn an_empty_seed_converges_in_zero_rounds() {
        let (all, record) = reach(&[], &[(0, 1)]);
        assert!(all.is_empty());
        assert_eq!(
            record,
            FixedPointRecord {
                iterations: 0,
                converged: true,
                delta_remaining: 0
            }
        );
    }

    #[test]
    fn hitting_the_round_bound_is_reported_as_not_converged_never_as_closed() {
        let (all, record) = semi_naive_fixed_point([0u64], 5, |delta, emit| {
            for n in delta {
                emit(n + 1);
            }
        });
        assert_eq!(
            record,
            FixedPointRecord {
                iterations: 5,
                converged: false,
                delta_remaining: 1
            }
        );
        assert_eq!(all, (0..=5).collect::<BTreeSet<u64>>());
    }
}
