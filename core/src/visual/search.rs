//! Multi-candidate design search: originality-valid recipe generation over a genome, measured
//! objectives, and Pareto preservation (ADR 0020).
//!
//! Candidates are `HYPOTHESIS` until created and verified; only verified candidates compete.
//! Objectives are measurements or explicit proxies -- never aesthetic truth -- and no single
//! "best" is declared when candidates trade off: the whole non-dominated set is kept.

use super::VisualObservationReport;
use super::compose::element;
use super::genome::{DesignGenome, MechanismKind, Recombination};
use crate::ConstraintVerdict;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Assumed mean glyph advance of the copy face, in `em` (a stated proxy for proportional sans
/// faces, not a measurement of the rendered glyphs).
pub const MEAN_ADVANCE_EM: f64 = 0.5;
/// Typographic line-length target in characters (Bringhurst's 66, within the 45-75 band).
pub const MEASURE_TARGET_CHARS: f64 = 66.0;

/// Candidate measurements taken from its re-observed render (ADR 0020).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidateMeasurements {
    /// Copy block width / (copy font size x `MEAN_ADVANCE_EM`) at the wide viewport.
    pub copy_chars_per_line: Option<f64>,
    /// Content extent at the narrow viewport (scroll burden): the lowest observed element
    /// bottom edge, px. Not `scrollHeight`, which is floored at the viewport height; absent when
    /// the observation was truncated (the extent would only be a lower bound).
    pub narrow_content_height_px: Option<f64>,
}

/// Measures the hero copy line length at `wide` and the document height at `narrow`.
pub fn measure(layout: &VisualObservationReport, wide: u32, narrow: u32) -> CandidateMeasurements {
    let copy_chars_per_line = element(layout, wide, "body>section:1>div:2").and_then(|copy| {
        let font_px = copy
            .style
            .get("font-size")?
            .strip_suffix("px")?
            .parse::<f64>()
            .ok()
            .filter(|px| *px > 0.0)?;
        Some(copy.width / (font_px * MEAN_ADVANCE_EM))
    });
    let narrow_content_height_px = layout
        .viewports
        .iter()
        .find(|v| v.width == narrow && !v.truncated)
        .and_then(|v| {
            v.elements
                .iter()
                .map(|e| e.y + e.height)
                .max_by(f64::total_cmp)
        });
    CandidateMeasurements {
        copy_chars_per_line,
        narrow_content_height_px,
    }
}

/// The objectives every search candidate is scored on, in score order.
pub fn design_objectives() -> Vec<Objective> {
    vec![
        Objective {
            name: "novelty".into(),
            direction: Direction::Maximize,
            meaning: "summed |ln(varied/source)| over varied parameters: distance from the \
                      references, not quality"
                .into(),
        },
        Objective {
            name: "measure_deviation".into(),
            direction: Direction::Minimize,
            meaning: "|estimated copy chars-per-line - 66| at the wide viewport: a line-length \
                      proxy (mean advance 0.5em assumed), not legibility"
                .into(),
        },
        Objective {
            name: "narrow_height_px".into(),
            direction: Direction::Minimize,
            meaning: "measured content extent at the narrow viewport: scroll burden".into(),
        },
    ]
}

/// Scores in `design_objectives` order; `None` when any measurement is missing (such a candidate
/// cannot compete -- an unmeasured objective is not a zero).
pub fn score(
    genome: &DesignGenome,
    recipe: &Recombination,
    measured: &CandidateMeasurements,
) -> Option<Vec<f64>> {
    Some(vec![
        novelty(genome, recipe),
        (measured.copy_chars_per_line? - MEASURE_TARGET_CHARS).abs(),
        measured.narrow_content_height_px?,
    ])
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Direction {
    Maximize,
    Minimize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Objective {
    pub name: String,
    pub direction: Direction,
    /// What the number is and is not (e.g. "proxy: chars-per-line estimate, not legibility").
    pub meaning: String,
}

/// `a` dominates `b`: at least as good on every objective and strictly better on one.
pub fn dominates(a: &[f64], b: &[f64], objectives: &[Objective]) -> bool {
    let mut strictly = false;
    for ((x, y), objective) in a.iter().zip(b).zip(objectives) {
        let (better, worse) = match objective.direction {
            Direction::Maximize => (x > y, x < y),
            Direction::Minimize => (x < y, x > y),
        };
        if worse {
            return false;
        }
        strictly |= better;
    }
    strictly
}

/// Indices of the non-dominated scores (the Pareto front), in input order.
pub fn pareto_front(scores: &[Vec<f64>], objectives: &[Objective]) -> Vec<usize> {
    (0..scores.len())
        .filter(|&i| {
            !(0..scores.len()).any(|j| j != i && dominates(&scores[j], &scores[i], objectives))
        })
        .collect()
}

/// Indices (into `candidates`) of the Pareto front among the candidates that are verified
/// (`Satisfied`) and fully measured. Unverified candidates stay `HYPOTHESIS` and never compete,
/// however good their numbers look.
pub fn verified_front(
    candidates: &[(ConstraintVerdict, Option<Vec<f64>>)],
    objectives: &[Objective],
) -> Vec<usize> {
    let competing: Vec<(usize, &Vec<f64>)> = candidates
        .iter()
        .enumerate()
        .filter(|(_, (verdict, _))| verdict.admits())
        .filter_map(|(i, (_, scores))| Some((i, scores.as_ref()?)))
        .collect();
    let scores: Vec<Vec<f64>> = competing.iter().map(|(_, s)| (*s).clone()).collect();
    pareto_front(&scores, objectives)
        .into_iter()
        .map(|k| competing[k].0)
        .collect()
}

/// Deterministically enumerates up to `count` originality-valid recipes: every candidate draws
/// its hero, grid and lift from the first mechanism of each kind (distinct sources required by
/// `recombine`) and varies one parameter of each from fixed option ladders.
pub fn generate_recipes(genome: &DesignGenome, count: usize) -> Vec<Recombination> {
    let first = |kind| genome.mechanisms.iter().find(|m| m.kind == kind);
    let (Some(hero), Some(grid), Some(lift)) = (
        first(MechanismKind::SplitHero),
        first(MechanismKind::CollapsingGrid),
        first(MechanismKind::HoverLift),
    ) else {
        return Vec::new();
    };
    let hero_options = [
        ("hero.media_aspect", "3:2"),
        ("hero.media_aspect", "21:9"),
        ("hero.share", "1:1"),
    ];
    let grid_options = [
        ("grid.columns", "4"),
        ("grid.columns", "2"),
        ("grid.item_aspect", "1:1"),
    ];
    let lift_options = [
        ("lift.lift_px", "8"),
        ("lift.duration_ms", "240"),
        ("lift.lift_px", "2"),
    ];
    let mut recipes = Vec::new();
    // Diagonal-first traversal of the 3x3x3 option cube, so small counts still vary every axis;
    // each (hero, grid, lift) triple is visited once, and distinct triples are distinct recipes.
    'outer: for shift in 0..9 {
        for i in 0..3 {
            let (h, g, l) = (i, (i + shift) % 3, (i + shift / 3) % 3);
            let mut variations = BTreeMap::new();
            for (key, value) in [hero_options[h], grid_options[g], lift_options[l]] {
                variations.insert(key.to_owned(), value.to_owned());
            }
            if variations.contains_key("grid.columns") {
                // Keep a whole number of rows: items follow the column count.
                let columns = variations["grid.columns"].clone();
                variations.insert("grid.items".into(), columns);
            }
            let recipe = Recombination {
                name: format!("candidate-{}", recipes.len() + 1),
                hero: hero.id.clone(),
                grid: grid.id.clone(),
                lift: lift.id.clone(),
                variations,
            };
            recipes.push(recipe);
            if recipes.len() == count {
                break 'outer;
            }
        }
    }
    recipes
}

/// Novelty of a recombination: summed |log(new / source)| over its varied numeric parameters
/// (ratios compared as values). A distance from the sources, not a judgement of quality.
pub fn novelty(genome: &DesignGenome, recipe: &Recombination) -> f64 {
    let value = |text: &str| -> Option<f64> {
        match text.split_once(':') {
            Some((p, q)) => Some(p.trim().parse::<f64>().ok()? / q.trim().parse::<f64>().ok()?),
            None => text.trim().parse().ok(),
        }
    };
    let source_of = |group: &str| {
        let id = match group {
            "hero" => &recipe.hero,
            "grid" => &recipe.grid,
            _ => &recipe.lift,
        };
        genome.mechanisms.iter().find(|m| &m.id == id)
    };
    recipe
        .variations
        .iter()
        .filter_map(|(key, varied)| {
            let (group, name) = key.split_once('.')?;
            let source = source_of(group)?.parameters.get(name)?;
            Some((value(varied)? / value(source)?).ln().abs())
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::super::genome::{DesignMechanism, MechanismSource};
    use super::*;
    use crate::{EpistemicStatus, IntegrityDigest};

    fn objectives() -> Vec<Objective> {
        vec![
            Objective {
                name: "novelty".into(),
                direction: Direction::Maximize,
                meaning: "".into(),
            },
            Objective {
                name: "height".into(),
                direction: Direction::Minimize,
                meaning: "".into(),
            },
        ]
    }

    #[test]
    fn the_front_keeps_every_trade_off_and_drops_only_dominated_candidates() {
        let scores = vec![
            vec![1.0, 900.0], // novel but tall
            vec![0.2, 500.0], // short but familiar
            vec![0.5, 950.0], // dominated by the first
            vec![1.0, 900.0], // tie with the first: neither dominates
        ];
        assert_eq!(pareto_front(&scores, &objectives()), vec![0, 1, 3]);
        assert!(dominates(&scores[0], &scores[2], &objectives()));
        assert!(!dominates(&scores[0], &scores[1], &objectives()));
        assert!(!dominates(&scores[0], &scores[3], &objectives()));
    }

    #[test]
    fn only_verified_and_measured_candidates_compete() {
        use ConstraintVerdict::{Satisfied, Unknown, Violated};
        let candidates = vec![
            (Violated, Some(vec![9.0, 1.0])), // would dominate everything, but failed verification
            (Satisfied, Some(vec![1.0, 900.0])),
            (Unknown, Some(vec![9.0, 1.0])), // unverifiable is not verified
            (Satisfied, None),               // verified but unmeasured
            (Satisfied, Some(vec![0.5, 950.0])), // dominated by candidate 1
            (Satisfied, Some(vec![0.2, 500.0])),
        ];
        assert_eq!(verified_front(&candidates, &objectives()), vec![1, 5]);
        assert!(verified_front(&[], &objectives()).is_empty());
    }

    fn genome() -> DesignGenome {
        let mechanism =
            |kind, id: &str, digest: &[u8], parameters: &[(&str, &str)]| DesignMechanism {
                id: id.into(),
                kind,
                parameters: parameters
                    .iter()
                    .map(|(k, v)| ((*k).into(), (*v).into()))
                    .collect(),
                source: MechanismSource {
                    locator: "r".into(),
                    subject_digest: IntegrityDigest::of_bytes(digest),
                    evidence: vec![],
                },
                status: EpistemicStatus::Derived,
            };
        DesignGenome {
            schema: "atlas.design-genome.v1".into(),
            mechanisms: vec![
                mechanism(
                    MechanismKind::SplitHero,
                    "h",
                    b"A",
                    &[
                        ("media_aspect", "16:9"),
                        ("share", "2:1"),
                        ("breakpoint_px", "700"),
                    ],
                ),
                mechanism(
                    MechanismKind::CollapsingGrid,
                    "g",
                    b"A",
                    &[
                        ("columns", "3"),
                        ("items", "3"),
                        ("item_aspect", "4:3"),
                        ("breakpoint_px", "700"),
                    ],
                ),
                mechanism(
                    MechanismKind::HoverLift,
                    "l",
                    b"B",
                    &[
                        ("lift_px", "4"),
                        ("easing", "ease-out"),
                        ("duration_ms", "150"),
                    ],
                ),
            ],
        }
    }

    #[test]
    fn generated_recipes_are_distinct_and_every_one_recombines() {
        let genome = genome();
        let recipes = generate_recipes(&genome, 6);
        assert_eq!(recipes.len(), 6);
        for (i, recipe) in recipes.iter().enumerate() {
            assert!(
                super::super::genome::recombine(&genome, recipe).is_ok(),
                "{recipe:?}"
            );
            assert!(
                recipes[..i]
                    .iter()
                    .all(|other| other.variations != recipe.variations)
            );
        }
        assert_eq!(generate_recipes(&genome, 6), recipes, "deterministic");
        let all = generate_recipes(&genome, 100);
        assert_eq!(all.len(), 27, "the whole option cube, once");
        for (i, recipe) in all.iter().enumerate() {
            assert!(all[..i].iter().all(|o| o.variations != recipe.variations));
        }
    }

    #[test]
    fn measurements_come_from_the_named_viewports_and_missing_ones_cannot_compete() {
        use super::super::tests::{element, report, viewport};
        let copy = |width| {
            element(
                "body>section:1>div:2",
                Some("body>section:1"),
                [0.0, 0.0, width, 100.0],
                &[("font-size", "24px")],
            )
        };
        let footer = element(
            "body>footer:1",
            Some("body"),
            [0.0, 2000.0, 400.0, 100.0],
            &[],
        );
        let mut narrow = viewport(400, vec![copy(400.0), footer]);
        // scrollHeight is floored at the viewport; the content extent is not.
        narrow.document_height = 5000.0;
        let layout = report(vec![viewport(1280, vec![copy(720.0)]), narrow.clone()]);
        let measured = measure(&layout, 1280, 400);
        // 720 px / (24 px x 0.5) = 60 characters, taken at the wide viewport only.
        assert_eq!(measured.copy_chars_per_line, Some(60.0));
        assert_eq!(measured.narrow_content_height_px, Some(2100.0));
        let genome = genome();
        let recipe = &generate_recipes(&genome, 1)[0];
        let scores = score(&genome, recipe, &measured).unwrap();
        assert_eq!(scores[1], 6.0);
        assert_eq!(scores[2], 2100.0);
        assert_eq!(scores.len(), design_objectives().len());
        let unmeasured = measure(&layout, 1280, 375);
        assert_eq!(unmeasured.narrow_content_height_px, None);
        assert_eq!(score(&genome, recipe, &unmeasured), None);
        narrow.truncated = true;
        let truncated = report(vec![viewport(1280, vec![copy(720.0)]), narrow]);
        assert_eq!(
            measure(&truncated, 1280, 400).narrow_content_height_px,
            None
        );
    }

    #[test]
    fn novelty_is_the_log_distance_from_source_values() {
        let genome = genome();
        let recipe = Recombination {
            name: "n".into(),
            hero: "h".into(),
            grid: "g".into(),
            lift: "l".into(),
            variations: [
                ("hero.media_aspect", "3:2"),
                ("grid.columns", "6"),
                ("lift.lift_px", "4"),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect(),
        };
        let expected = ((1.5_f64) / (16.0 / 9.0)).ln().abs() + 2.0_f64.ln() + 0.0;
        assert!((novelty(&genome, &recipe) - expected).abs() < 1e-12);
    }
}
