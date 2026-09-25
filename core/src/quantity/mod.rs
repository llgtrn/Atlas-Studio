//! Physical quantities with dimensional correctness (ADR 0010).
//!
//! A `Quantity` is an exact rational value in coherent SI base units plus a `Dimension`: the
//! exponent vector over eight base dimensions -- the seven SI bases (length, mass, time, current,
//! temperature, amount, luminous intensity) plus plane angle. Angle is kept as its own base, unlike
//! SI's "dimensionless radian", so an angle can never be silently added to a length ratio.
//!
//! Exactness is deliberate: `0.12 m` and `120 mm` are the same quantity, compared without floating
//! point. Only units whose SI factor is an exact rational are admitted; a unit that needs an
//! irrational factor (`deg` = pi/180 rad, `rpm`) or an affine offset (`degC`, `degF`) is
//! `UnsupportedUnit` -- reported, never approximated. Arithmetic that overflows the exact
//! representation reports `Overflow` rather than rounding.
//!
//! An exponent vector cannot tell dimensionally equal but physically distinct quantities apart
//! (torque N*m vs energy J; frequency Hz vs becquerel). A `QuantityKind` does (G124, ADR 0045): a
//! kind is named by its unit (`J`, `Hz`, `Bq`) or declared (`5 N*m torque`), and two quantities of
//! different declared kinds never add, subtract or compare. A kind that was never declared is
//! `Unspecified` and joins any kind of its dimension.
//!
//! Uncertainty is an exact interval (`120 ± 0.5 mm`): `uncertainty` bounds the true value in SI
//! units, and every operation propagates exact interval bounds, so a derived interval always
//! contains the exact result of any operands inside their intervals. Comparing overlapping
//! intervals is `Undecided`, never guessed.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;

/// Base-dimension order of `Dimension::exponents`.
pub const BASE_DIMENSIONS: [&str; 8] = [
    "length",
    "mass",
    "time",
    "current",
    "temperature",
    "amount",
    "luminous_intensity",
    "angle",
];

const BASE_SYMBOLS: [&str; 8] = ["m", "kg", "s", "A", "K", "mol", "cd", "rad"];

#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct Dimension {
    pub exponents: [i8; 8],
}

impl Dimension {
    pub const DIMENSIONLESS: Self = Self { exponents: [0; 8] };

    const fn base(index: usize) -> Self {
        let mut exponents = [0; 8];
        exponents[index] = 1;
        Self { exponents }
    }

    fn combine(self, other: Self, sign: i8) -> Option<Self> {
        let mut exponents = [0; 8];
        for (i, slot) in exponents.iter_mut().enumerate() {
            *slot = self.exponents[i].checked_add(other.exponents[i].checked_mul(sign)?)?;
        }
        Some(Self { exponents })
    }

    fn pow(self, power: i8) -> Option<Self> {
        let mut exponents = [0; 8];
        for (i, slot) in exponents.iter_mut().enumerate() {
            *slot = self.exponents[i].checked_mul(power)?;
        }
        Some(Self { exponents })
    }
}

impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self
            .exponents
            .iter()
            .zip(BASE_SYMBOLS)
            .filter(|(exponent, _)| **exponent != 0)
            .map(|(exponent, symbol)| match exponent {
                1 => symbol.to_owned(),
                n => format!("{symbol}^{n}"),
            })
            .collect();
        if parts.is_empty() {
            f.write_str("1")
        } else {
            f.write_str(&parts.join("*"))
        }
    }
}

/// What a quantity is beyond its dimension. `Unspecified` joins any kind of its dimension; two
/// different declared kinds never combine.
#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QuantityKind {
    #[default]
    Unspecified,
    Energy,
    Torque,
    Frequency,
    Activity,
}

impl QuantityKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Unspecified => "UNSPECIFIED",
            Self::Energy => "ENERGY",
            Self::Torque => "TORQUE",
            Self::Frequency => "FREQUENCY",
            Self::Activity => "ACTIVITY",
        }
    }

    fn declared(word: &str) -> Option<Self> {
        Some(match word {
            "energy" => Self::Energy,
            "torque" => Self::Torque,
            "frequency" => Self::Frequency,
            "activity" => Self::Activity,
            _ => return None,
        })
    }

    /// The dimension a declared kind requires.
    pub const fn dimension(self) -> Option<Dimension> {
        match self {
            Self::Unspecified => None,
            Self::Energy | Self::Torque => Some(Dimension {
                exponents: [2, 1, -2, 0, 0, 0, 0, 0],
            }),
            Self::Frequency | Self::Activity => Some(Dimension {
                exponents: [0, 0, -1, 0, 0, 0, 0, 0],
            }),
        }
    }

    /// The kind two operands share, or `None` when they are different declared kinds.
    pub fn join(self, other: Self) -> Option<Self> {
        match (self, other) {
            (a, b) if a == b => Some(a),
            (Self::Unspecified, b) => Some(b),
            (a, Self::Unspecified) => Some(a),
            _ => None,
        }
    }
}

/// Exact bounds `[low, high]`, in SI units, that contain the true value.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Interval {
    pub low: Rational,
    pub high: Rational,
}

impl Interval {
    /// G131 (NA-UNCERTAINTY-PREDICTIONS): the one exact uncertainty carrier -- a quantity's bounds
    /// and product money are both this interval.
    pub fn point(value: Rational) -> Self {
        Self {
            low: value,
            high: value,
        }
    }

    /// The hull of `values`; `None` for no values or an overflowing comparison.
    pub fn hull(values: &[Rational]) -> Option<Self> {
        let (low, high) = Rational::min_max(values)?;
        Some(Self { low, high })
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        Some(Self {
            low: self.low.checked_add(other.low)?,
            high: self.high.checked_add(other.high)?,
        })
    }

    pub fn checked_sub(self, other: Self) -> Option<Self> {
        Some(Self {
            low: self.low.checked_add(other.high.checked_neg()?)?,
            high: self.high.checked_add(other.low.checked_neg()?)?,
        })
    }

    /// The exact product: the hull of the four corner products, whatever the signs.
    pub fn checked_mul(self, other: Self) -> Option<Self> {
        Self::hull(&[
            self.low.checked_mul(other.low)?,
            self.low.checked_mul(other.high)?,
            self.high.checked_mul(other.low)?,
            self.high.checked_mul(other.high)?,
        ])
    }

    /// Scales by an exact factor; a negative factor swaps the bounds.
    pub fn scale(self, factor: Rational) -> Option<Self> {
        Self::hull(&[
            self.low.checked_mul(factor)?,
            self.high.checked_mul(factor)?,
        ])
    }

    pub fn is_nonnegative(self) -> bool {
        self.low.checked_cmp(Rational::ZERO) != Some(Ordering::Less)
    }

    /// Where `value` lies relative to the interval: `Less` below it, `Greater` above it, `Equal`
    /// inside it (bounds included); `None` on overflow.
    pub fn locate(self, value: Rational) -> Option<Ordering> {
        if value.checked_cmp(self.low)? == Ordering::Less {
            Some(Ordering::Less)
        } else if value.checked_cmp(self.high)? == Ordering::Greater {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Equal)
        }
    }
}

/// G131: a declared value that left exact arithmetic for `f64` -- what it was, the `f64` used,
/// whether that `f64` is exact, and the declared bounds the `f64` did not carry along. Every such
/// drop in a verdict path is recorded; a verdict never rests silently on a nominal value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FloatDrop {
    pub subject: String,
    pub value: f64,
    pub exact: bool,
    /// The declared uncertainty, approximated: the `f64` above is only its nominal value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dropped_bounds: Option<(f64, f64)>,
}

/// An `f64` stand-in for an exact rational, marked with whether it is exact (G124): a value that
/// leaves exact arithmetic says so.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Approximation {
    pub value: f64,
    pub exact: bool,
}

/// An exact rational `numerator / denominator`, always normalized: denominator > 0, lowest terms.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Rational {
    numerator: i128,
    denominator: i128,
}

fn gcd(mut a: i128, mut b: i128) -> i128 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

impl Rational {
    pub const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };
    pub const ONE: Self = Self {
        numerator: 1,
        denominator: 1,
    };

    pub fn new(numerator: i128, denominator: i128) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        let divisor = gcd(numerator, denominator).max(1);
        let sign = if denominator < 0 { -1 } else { 1 };
        Some(Self {
            numerator: (numerator / divisor).checked_mul(sign)?,
            denominator: (denominator / divisor).checked_mul(sign)?,
        })
    }

    pub fn integer(value: i128) -> Self {
        Self {
            numerator: value,
            denominator: 1,
        }
    }

    pub fn numerator(self) -> i128 {
        self.numerator
    }

    pub fn denominator(self) -> i128 {
        self.denominator
    }

    pub fn checked_mul(self, other: Self) -> Option<Self> {
        // Cross-reduce first so exact products of large-but-cancelling factors do not overflow.
        let g1 = gcd(self.numerator, other.denominator).max(1);
        let g2 = gcd(other.numerator, self.denominator).max(1);
        Self::new(
            (self.numerator / g1).checked_mul(other.numerator / g2)?,
            (self.denominator / g2).checked_mul(other.denominator / g1)?,
        )
    }

    pub fn checked_div(self, other: Self) -> Option<Self> {
        if other.numerator == 0 {
            return None;
        }
        self.checked_mul(Self::new(other.denominator, other.numerator)?)
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        let g = gcd(self.denominator, other.denominator).max(1);
        let left = self.numerator.checked_mul(other.denominator / g)?;
        let right = other.numerator.checked_mul(self.denominator / g)?;
        Self::new(
            left.checked_add(right)?,
            (self.denominator / g).checked_mul(other.denominator)?,
        )
    }

    pub fn checked_neg(self) -> Option<Self> {
        Some(Self {
            numerator: self.numerator.checked_neg()?,
            denominator: self.denominator,
        })
    }

    fn checked_pow(self, power: i8) -> Option<Self> {
        let mut result = Self::ONE;
        for _ in 0..power.unsigned_abs() {
            result = result.checked_mul(self)?;
        }
        if power < 0 {
            Self::ONE.checked_div(result)
        } else {
            Some(result)
        }
    }

    /// Exact ordering (cross-multiplied); `None` only if the comparison itself would overflow.
    pub fn checked_cmp(self, other: Self) -> Option<Ordering> {
        let left = self.numerator.checked_mul(other.denominator)?;
        let right = other.numerator.checked_mul(self.denominator)?;
        Some(left.cmp(&right))
    }

    /// Nearest `f64`, for display and non-exact consumers only; `approximate` marks exactness.
    pub fn to_f64(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    /// The nearest `f64` and whether it is exact: only a dyadic rational whose numerator fits the
    /// 53-bit significand survives the conversion unchanged.
    pub fn approximate(self) -> Approximation {
        let dyadic = self.denominator & (self.denominator - 1) == 0;
        let exact =
            dyadic && self.numerator.unsigned_abs() <= 1 << 53 && self.denominator <= 1 << 60;
        Approximation {
            value: self.to_f64(),
            exact,
        }
    }

    fn min_max(values: &[Self]) -> Option<(Self, Self)> {
        let mut low = *values.first()?;
        let mut high = low;
        for v in &values[1..] {
            if v.checked_cmp(low)? == Ordering::Less {
                low = *v;
            }
            if v.checked_cmp(high)? == Ordering::Greater {
                high = *v;
            }
        }
        Some((low, high))
    }

    /// Parses a decimal literal exactly: optional sign, digits, optional fraction, optional
    /// `e`/`E` exponent. `0.1` is exactly 1/10.
    pub fn parse_decimal(text: &str) -> Option<Self> {
        let (mantissa, exponent) = match text.find(['e', 'E']) {
            Some(at) => (&text[..at], text[at + 1..].parse::<i32>().ok()?),
            None => (text, 0),
        };
        let (negative, digits) = match mantissa.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, mantissa.strip_prefix('+').unwrap_or(mantissa)),
        };
        let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
        if whole.is_empty() && fraction.is_empty()
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || !fraction.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        let mut numerator: i128 = 0;
        for byte in whole.bytes().chain(fraction.bytes()) {
            numerator = numerator
                .checked_mul(10)?
                .checked_add(i128::from(byte - b'0'))?;
        }
        let scale = exponent.checked_sub(i32::try_from(fraction.len()).ok()?)?;
        if scale.unsigned_abs() > 36 {
            return None;
        }
        let power = 10_i128.checked_pow(scale.unsigned_abs())?;
        let value = if scale >= 0 {
            Self::new(numerator.checked_mul(power)?, 1)?
        } else {
            Self::new(numerator, power)?
        };
        if negative {
            value.checked_neg()
        } else {
            Some(value)
        }
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denominator == 1 {
            write!(f, "{}", self.numerator)
        } else {
            write!(f, "{}/{}", self.numerator, self.denominator)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "error")]
pub enum QuantityError {
    /// Not of the form `<decimal number> <unit expression>`.
    NotAQuantity,
    /// A syntactically valid unit symbol Atlas does not admit (irrational/affine factor or
    /// simply unknown). Never approximated.
    UnsupportedUnit { unit: String },
    /// An operation combined quantities of different dimensions (e.g. `3 kg + 2 V`).
    DimensionMismatch { left: Dimension, right: Dimension },
    /// The exact result does not fit the rational representation.
    Overflow,
    /// Two different declared kinds of one dimension were combined (torque + energy).
    KindMismatch {
        left: QuantityKind,
        right: QuantityKind,
    },
    /// A declared kind whose dimension is not the quantity's (`3 kg torque`).
    KindDimensionMismatch {
        kind: QuantityKind,
        dimension: Dimension,
    },
    /// Two uncertain quantities whose intervals overlap: their order is not decided.
    Undecided,
    /// A divisor whose interval contains zero.
    UncertainDivisor,
    /// A tolerance that is negative, or bounds that do not contain the value.
    InvalidUncertainty,
}

impl fmt::Display for QuantityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAQuantity => f.write_str("not a quantity"),
            Self::UnsupportedUnit { unit } => write!(f, "unsupported unit `{unit}`"),
            Self::DimensionMismatch { left, right } => {
                write!(f, "dimension mismatch: {left} vs {right}")
            }
            Self::Overflow => f.write_str("exact arithmetic overflow"),
            Self::KindMismatch { left, right } => {
                write!(f, "kind mismatch: {} vs {}", left.as_str(), right.as_str())
            }
            Self::KindDimensionMismatch { kind, dimension } => {
                write!(
                    f,
                    "kind {} cannot have dimension {dimension}",
                    kind.as_str()
                )
            }
            Self::Undecided => f.write_str("uncertain quantities overlap: order undecided"),
            Self::UncertainDivisor => f.write_str("divisor interval contains zero"),
            Self::InvalidUncertainty => f.write_str("invalid uncertainty"),
        }
    }
}

/// A value in coherent SI base units with its dimension, kind and uncertainty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Quantity {
    pub si_value: Rational,
    pub dimension: Dimension,
    #[serde(default)]
    pub kind: QuantityKind,
    /// Exact bounds containing the true value; `None` for an exact quantity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertainty: Option<Interval>,
}

/// (symbol, SI factor as numerator/denominator, dimension) for every admitted unprefixed unit.
fn named_unit(symbol: &str) -> Option<(Rational, Dimension)> {
    let d = |exponents: [i8; 8]| Dimension { exponents };
    let r = |n: i128, m: i128| Rational::new(n, m).expect("nonzero literal denominator");
    Some(match symbol {
        "m" => (Rational::ONE, Dimension::base(0)),
        "g" => (r(1, 1000), Dimension::base(1)),
        "s" => (Rational::ONE, Dimension::base(2)),
        "A" => (Rational::ONE, Dimension::base(3)),
        "K" => (Rational::ONE, Dimension::base(4)),
        "mol" => (Rational::ONE, Dimension::base(5)),
        "cd" => (Rational::ONE, Dimension::base(6)),
        "rad" => (Rational::ONE, Dimension::base(7)),
        "min" => (r(60, 1), Dimension::base(2)),
        "h" => (r(3600, 1), Dimension::base(2)),
        "L" => (r(1, 1000), d([3, 0, 0, 0, 0, 0, 0, 0])),
        "Hz" | "Bq" => (Rational::ONE, d([0, 0, -1, 0, 0, 0, 0, 0])),
        "N" => (Rational::ONE, d([1, 1, -2, 0, 0, 0, 0, 0])),
        "Pa" => (Rational::ONE, d([-1, 1, -2, 0, 0, 0, 0, 0])),
        "J" => (Rational::ONE, d([2, 1, -2, 0, 0, 0, 0, 0])),
        "W" => (Rational::ONE, d([2, 1, -3, 0, 0, 0, 0, 0])),
        "C" => (Rational::ONE, d([0, 0, 1, 1, 0, 0, 0, 0])),
        "V" => (Rational::ONE, d([2, 1, -3, -1, 0, 0, 0, 0])),
        "ohm" | "Ω" => (Rational::ONE, d([2, 1, -3, -2, 0, 0, 0, 0])),
        "S" => (Rational::ONE, d([-2, -1, 3, 2, 0, 0, 0, 0])),
        "F" => (Rational::ONE, d([-2, -1, 4, 2, 0, 0, 0, 0])),
        "H" => (Rational::ONE, d([2, 1, -2, -2, 0, 0, 0, 0])),
        "Wb" => (Rational::ONE, d([2, 1, -2, -1, 0, 0, 0, 0])),
        "T" => (Rational::ONE, d([0, 1, -2, -1, 0, 0, 0, 0])),
        _ => return None,
    })
}

fn prefix_factor(prefix: &str) -> Option<Rational> {
    let power: i32 = match prefix {
        "T" => 12,
        "G" => 9,
        "M" => 6,
        "k" => 3,
        "h" => 2,
        "da" => 1,
        "d" => -1,
        "c" => -2,
        "m" => -3,
        "u" | "µ" | "μ" => -6,
        "n" => -9,
        "p" => -12,
        "f" => -15,
        _ => return None,
    };
    let magnitude = 10_i128.pow(power.unsigned_abs());
    if power >= 0 {
        Rational::new(magnitude, 1)
    } else {
        Rational::new(1, magnitude)
    }
}

/// Resolves one unit symbol: an exact unprefixed name wins (`min`, `cd`, `h`, `T`, `Pa`), then the
/// longest SI prefix followed by a prefixable unit (`mm`, `kΩ`, `µF`). Time units other than the
/// second (`min`, `h`) are not prefixable; the litre is (`mL`).
fn unit_symbol(symbol: &str) -> Result<(Rational, Dimension), QuantityError> {
    if let Some(unit) = named_unit(symbol) {
        return Ok(unit);
    }
    for split in symbol.char_indices().map(|(at, _)| at).skip(1) {
        let (prefix, rest) = symbol.split_at(split);
        if matches!(rest, "min" | "h") {
            continue;
        }
        if let (Some(factor), Some((unit_factor, dimension))) =
            (prefix_factor(prefix), named_unit(rest))
        {
            let combined = factor
                .checked_mul(unit_factor)
                .ok_or(QuantityError::Overflow)?;
            return Ok((combined, dimension));
        }
    }
    Err(QuantityError::UnsupportedUnit {
        unit: symbol.to_owned(),
    })
}

/// The kind a single unit symbol names: the joule names energy, the hertz frequency, the
/// becquerel activity (any SI prefix). Compound expressions (`N*m`, `s^-1`) name no kind.
fn unit_kind(unit: &str) -> QuantityKind {
    let named = |symbol: &str| match symbol {
        "J" => Some(QuantityKind::Energy),
        "Hz" => Some(QuantityKind::Frequency),
        "Bq" => Some(QuantityKind::Activity),
        _ => None,
    };
    if let Some(kind) = named(unit) {
        return kind;
    }
    unit.char_indices()
        .map(|(at, _)| at)
        .skip(1)
        .find_map(|at| {
            let (prefix, rest) = unit.split_at(at);
            prefix_factor(prefix).and(named(rest))
        })
        .unwrap_or_default()
}

/// Parses a unit expression: unit terms joined by `*`, `·` or `/`, each with an optional integer
/// power `^n` (`m/s^2`, `N*m`, `kg*m^2`, `V/A`). Division applies to the single following term.
pub fn parse_unit_expression(text: &str) -> Result<(Rational, Dimension), QuantityError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(QuantityError::NotAQuantity);
    }
    let mut factor = Rational::ONE;
    let mut dimension = Dimension::DIMENSIONLESS;
    let mut divide = false;
    let mut term = String::new();
    let flush = |term: &str,
                 divide: bool,
                 factor: &mut Rational,
                 dimension: &mut Dimension|
     -> Result<(), QuantityError> {
        let (symbol, power) = match term.split_once('^') {
            Some((symbol, power)) => (
                symbol,
                power
                    .parse::<i8>()
                    .map_err(|_| QuantityError::UnsupportedUnit {
                        unit: term.to_owned(),
                    })?,
            ),
            None => (term, 1),
        };
        if symbol.is_empty() {
            return Err(QuantityError::NotAQuantity);
        }
        let (unit_factor, unit_dimension) = unit_symbol(symbol)?;
        let power = if divide { -power } else { power };
        let scaled = unit_factor
            .checked_pow(power)
            .ok_or(QuantityError::Overflow)?;
        *factor = factor.checked_mul(scaled).ok_or(QuantityError::Overflow)?;
        let raised = unit_dimension.pow(power).ok_or(QuantityError::Overflow)?;
        *dimension = dimension
            .combine(raised, 1)
            .ok_or(QuantityError::Overflow)?;
        Ok(())
    };
    for ch in text.chars() {
        match ch {
            '*' | '·' | '/' => {
                flush(&term, divide, &mut factor, &mut dimension)?;
                term.clear();
                divide = ch == '/';
            }
            c if c.is_whitespace() => return Err(QuantityError::NotAQuantity),
            c => term.push(c),
        }
    }
    flush(&term, divide, &mut factor, &mut dimension)?;
    Ok((factor, dimension))
}

impl Quantity {
    /// An exact quantity of no declared kind.
    pub fn new(si_value: Rational, dimension: Dimension) -> Self {
        Self {
            si_value,
            dimension,
            kind: QuantityKind::Unspecified,
            uncertainty: None,
        }
    }

    /// The same quantity with a declared kind, refused when the kind's dimension differs or the
    /// quantity already declares another kind (an energy never becomes a torque).
    pub fn with_kind(self, kind: QuantityKind) -> Result<Self, QuantityError> {
        let kind = self.kind.join(kind).ok_or(QuantityError::KindMismatch {
            left: self.kind,
            right: kind,
        })?;
        match kind.dimension() {
            Some(required) if required != self.dimension => {
                Err(QuantityError::KindDimensionMismatch {
                    kind,
                    dimension: self.dimension,
                })
            }
            _ => Ok(Self { kind, ..self }),
        }
    }

    /// The same quantity bounded by `[low, high]`, which must contain the value.
    pub fn with_bounds(self, low: Rational, high: Rational) -> Result<Self, QuantityError> {
        let contains = low
            .checked_cmp(self.si_value)
            .ok_or(QuantityError::Overflow)?
            != Ordering::Greater
            && self
                .si_value
                .checked_cmp(high)
                .ok_or(QuantityError::Overflow)?
                != Ordering::Greater;
        if !contains {
            return Err(QuantityError::InvalidUncertainty);
        }
        let uncertainty = (low != high || low != self.si_value).then_some(Interval { low, high });
        Ok(Self {
            uncertainty,
            ..self
        })
    }

    /// The exact bounds of the true value: the interval, or the value itself when exact.
    pub fn bounds(&self) -> (Rational, Rational) {
        self.uncertainty
            .map_or((self.si_value, self.si_value), |i| (i.low, i.high))
    }

    /// Parses `<decimal> [± <decimal>] <unit expression> [<kind>]`, e.g. `120 mm`, `3.3 V`,
    /// `9.81 m/s^2`, `120 ± 0.5 mm`, `5 N*m torque`. A bare number is `NotAQuantity`: a quantity
    /// always states its unit. The tolerance is in the same unit as the value.
    pub fn parse(text: &str) -> Result<Self, QuantityError> {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let (number, tolerance, unit, declared) = match tokens.as_slice() {
            [number, unit] => (*number, None, *unit, None),
            [number, sign, tolerance, unit] if matches!(*sign, "±" | "+/-") => {
                (*number, Some(*tolerance), *unit, None)
            }
            [number, unit, kind] => (*number, None, *unit, Some(*kind)),
            [number, sign, tolerance, unit, kind] if matches!(*sign, "±" | "+/-") => {
                (*number, Some(*tolerance), *unit, Some(*kind))
            }
            _ => return Err(QuantityError::NotAQuantity),
        };
        let value = Rational::parse_decimal(number).ok_or(QuantityError::NotAQuantity)?;
        let (factor, dimension) = parse_unit_expression(unit)?;
        let si_value = value.checked_mul(factor).ok_or(QuantityError::Overflow)?;
        let named = unit_kind(unit);
        let kind = match declared {
            None => named,
            Some(word) => {
                let declared = QuantityKind::declared(word).ok_or(QuantityError::NotAQuantity)?;
                named.join(declared).ok_or(QuantityError::KindMismatch {
                    left: named,
                    right: declared,
                })?
            }
        };
        let mut quantity = Self::new(si_value, dimension).with_kind(kind)?;
        if let Some(tolerance) = tolerance {
            let tolerance = Rational::parse_decimal(tolerance)
                .ok_or(QuantityError::NotAQuantity)?
                .checked_mul(factor)
                .ok_or(QuantityError::Overflow)?;
            if tolerance.checked_cmp(Rational::ZERO) == Some(Ordering::Less) {
                return Err(QuantityError::InvalidUncertainty);
            }
            let low = si_value
                .checked_add(tolerance.checked_neg().ok_or(QuantityError::Overflow)?)
                .ok_or(QuantityError::Overflow)?;
            let high = si_value
                .checked_add(tolerance)
                .ok_or(QuantityError::Overflow)?;
            quantity = quantity.with_bounds(low, high)?;
        }
        Ok(quantity)
    }

    /// `true` when `text` has the shape of a quantity -- a decimal literal (optionally `± <decimal>`)
    /// followed by a unit token -- whether or not the unit is admitted. Plain words (`backend`) and
    /// bare numbers (`1`) are not quantity-shaped.
    pub fn is_quantity_shaped(text: &str) -> bool {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let unit = match tokens.as_slice() {
            [number, sign, tolerance, rest @ ..] if matches!(*sign, "±" | "+/-") => {
                (Rational::parse_decimal(number).is_some()
                    && Rational::parse_decimal(tolerance).is_some())
                .then(|| rest.first())
                .flatten()
            }
            [number, rest @ ..] => Rational::parse_decimal(number)
                .is_some()
                .then(|| rest.first())
                .flatten(),
            [] => None,
        };
        unit.and_then(|u| u.chars().next())
            .is_some_and(|c| c.is_alphabetic() || c == 'Ω' || c == 'µ')
    }

    fn same_dimension(&self, other: &Self) -> Result<(), QuantityError> {
        if self.dimension == other.dimension {
            Ok(())
        } else {
            Err(QuantityError::DimensionMismatch {
                left: self.dimension,
                right: other.dimension,
            })
        }
    }

    fn shared_kind(&self, other: &Self) -> Result<QuantityKind, QuantityError> {
        self.kind
            .join(other.kind)
            .ok_or(QuantityError::KindMismatch {
                left: self.kind,
                right: other.kind,
            })
    }

    /// `value` with the interval spanning `corners` when either operand is uncertain.
    fn derived(
        &self,
        other: &Self,
        value: Rational,
        dimension: Dimension,
        kind: QuantityKind,
        corners: &[Rational],
    ) -> Result<Self, QuantityError> {
        let quantity = Self {
            si_value: value,
            dimension,
            kind,
            uncertainty: None,
        };
        if self.uncertainty.is_none() && other.uncertainty.is_none() {
            return Ok(quantity);
        }
        let (low, high) = Rational::min_max(corners).ok_or(QuantityError::Overflow)?;
        quantity.with_bounds(low, high)
    }

    pub fn checked_add(&self, other: &Self) -> Result<Self, QuantityError> {
        self.same_dimension(other)?;
        let kind = self.shared_kind(other)?;
        let value = self
            .si_value
            .checked_add(other.si_value)
            .ok_or(QuantityError::Overflow)?;
        let bounds = self
            .interval()
            .checked_add(other.interval())
            .ok_or(QuantityError::Overflow)?;
        self.derived(
            other,
            value,
            self.dimension,
            kind,
            &[bounds.low, bounds.high],
        )
    }

    pub fn checked_sub(&self, other: &Self) -> Result<Self, QuantityError> {
        self.same_dimension(other)?;
        let kind = self.shared_kind(other)?;
        let value = other
            .si_value
            .checked_neg()
            .and_then(|negated| self.si_value.checked_add(negated))
            .ok_or(QuantityError::Overflow)?;
        let bounds = self
            .interval()
            .checked_sub(other.interval())
            .ok_or(QuantityError::Overflow)?;
        self.derived(
            other,
            value,
            self.dimension,
            kind,
            &[bounds.low, bounds.high],
        )
    }

    /// A product has no declared kind: torque times angle is not a torque.
    pub fn checked_mul(&self, other: &Self) -> Result<Self, QuantityError> {
        let dimension = self
            .dimension
            .combine(other.dimension, 1)
            .ok_or(QuantityError::Overflow)?;
        let value = self
            .si_value
            .checked_mul(other.si_value)
            .ok_or(QuantityError::Overflow)?;
        let bounds = self
            .interval()
            .checked_mul(other.interval())
            .ok_or(QuantityError::Overflow)?;
        self.derived(
            other,
            value,
            dimension,
            QuantityKind::Unspecified,
            &[bounds.low, bounds.high],
        )
    }

    pub fn checked_div(&self, other: &Self) -> Result<Self, QuantityError> {
        let (l2, h2) = other.bounds();
        if other.uncertainty.is_some()
            && l2.checked_cmp(Rational::ZERO) != Some(Ordering::Greater)
            && h2.checked_cmp(Rational::ZERO) != Some(Ordering::Less)
        {
            return Err(QuantityError::UncertainDivisor);
        }
        let div = |a: Rational, b: Rational| a.checked_div(b).ok_or(QuantityError::Overflow);
        let (l1, h1) = self.bounds();
        let dimension = self
            .dimension
            .combine(other.dimension, -1)
            .ok_or(QuantityError::Overflow)?;
        let value = div(self.si_value, other.si_value)?;
        let corners = [div(l1, l2)?, div(l1, h2)?, div(h1, l2)?, div(h1, h2)?];
        self.derived(other, value, dimension, QuantityKind::Unspecified, &corners)
    }

    /// Exact ordering of two quantities of one dimension and compatible kinds. Uncertain
    /// quantities are ordered only when their intervals are disjoint; overlap is `Undecided`.
    pub fn checked_cmp(&self, other: &Self) -> Result<Ordering, QuantityError> {
        self.same_dimension(other)?;
        self.shared_kind(other)?;
        let cmp = |a: Rational, b: Rational| a.checked_cmp(b).ok_or(QuantityError::Overflow);
        if self.uncertainty.is_none() && other.uncertainty.is_none() {
            return cmp(self.si_value, other.si_value);
        }
        let ((l1, h1), (l2, h2)) = (self.bounds(), other.bounds());
        if cmp(h1, l2)? == Ordering::Less {
            Ok(Ordering::Less)
        } else if cmp(l1, h2)? == Ordering::Greater {
            Ok(Ordering::Greater)
        } else {
            Err(QuantityError::Undecided)
        }
    }

    /// The SI value as `f64`, marked exact or approximate.
    pub fn approximate(&self) -> Approximation {
        self.si_value.approximate()
    }

    /// The SI value as `f64` for a computation that cannot stay exact (G131), recorded in
    /// `drops` with its exactness and the declared bounds it leaves behind.
    pub fn drop_to_f64(&self, subject: impl Into<String>, drops: &mut Vec<FloatDrop>) -> f64 {
        let approximation = self.approximate();
        drops.push(FloatDrop {
            subject: subject.into(),
            value: approximation.value,
            exact: approximation.exact,
            dropped_bounds: self
                .uncertainty
                .map(|Interval { low, high }| (low.to_f64(), high.to_f64())),
        });
        approximation.value
    }

    /// The exact bounds as the one uncertainty carrier.
    pub fn interval(&self) -> Interval {
        let (low, high) = self.bounds();
        Interval { low, high }
    }
}

impl fmt::Display for Quantity {
    /// `<SI value> <dimension>`, then ` in [low, high]` when uncertain and the declared kind.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.si_value, self.dimension)?;
        if let Some(Interval { low, high }) = self.uncertainty {
            write!(f, " in [{low}, {high}]")?;
        }
        if self.kind != QuantityKind::Unspecified {
            write!(f, " {}", self.kind.as_str().to_ascii_lowercase())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// G131: the one uncertainty carrier's arithmetic is exact and sign-aware.
    #[test]
    fn interval_arithmetic_is_exact_and_sign_aware() {
        let r = |n: i128| Rational::integer(n);
        let a = Interval {
            low: r(-1),
            high: r(2),
        };
        let b = Interval {
            low: r(3),
            high: r(4),
        };
        assert_eq!(
            a.checked_mul(b),
            Some(Interval {
                low: r(-4),
                high: r(8)
            })
        );
        assert_eq!(
            a.checked_add(b),
            Some(Interval {
                low: r(2),
                high: r(6)
            })
        );
        assert_eq!(
            a.checked_sub(b),
            Some(Interval {
                low: r(-5),
                high: r(-1)
            })
        );
        assert_eq!(
            b.scale(r(-2)),
            Some(Interval {
                low: r(-8),
                high: r(-6)
            })
        );
        assert!(b.is_nonnegative() && !a.is_nonnegative());
        assert_eq!(a.locate(r(-2)), Some(Ordering::Less));
        assert_eq!(a.locate(r(2)), Some(Ordering::Equal));
        assert_eq!(a.locate(r(3)), Some(Ordering::Greater));
        assert_eq!(
            Interval::point(r(5)),
            Interval {
                low: r(5),
                high: r(5)
            }
        );
        // A quantity's bounds are this carrier; dropping one to f64 records what was left behind.
        let q = Quantity::parse("0.4 ± 0.05 kg").unwrap();
        let mut drops = Vec::new();
        assert_eq!(q.drop_to_f64("m", &mut drops), 0.4);
        assert_eq!(drops[0].dropped_bounds, Some((0.35, 0.45)));
        assert!(!drops[0].exact);
        assert_eq!(q.interval().low, Rational::parse_decimal("0.35").unwrap());
    }

    fn q(text: &str) -> Quantity {
        Quantity::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"))
    }

    /// G124: torque and energy share a dimension but are different kinds; so are frequency and
    /// activity. A kind that was never declared joins either.
    #[test]
    fn torque_and_energy_are_distinct_kinds() {
        let energy = q("1 J");
        let torque = q("1 N*m torque");
        let unspecified = q("1 N*m");
        assert_eq!(energy.kind, QuantityKind::Energy, "the joule names energy");
        assert_eq!(q("2 kJ").kind, QuantityKind::Energy, "under any prefix");
        assert_eq!(torque.kind, QuantityKind::Torque);
        assert_eq!(unspecified.kind, QuantityKind::Unspecified);
        assert_ne!(energy, torque, "torque never compares equal to energy");
        let mismatch = QuantityError::KindMismatch {
            left: QuantityKind::Energy,
            right: QuantityKind::Torque,
        };
        assert_eq!(energy.checked_cmp(&torque), Err(mismatch.clone()));
        assert_eq!(energy.checked_add(&torque), Err(mismatch));
        assert!(torque.checked_sub(&energy).is_err());
        assert_eq!(energy.checked_cmp(&unspecified), Ok(Ordering::Equal));
        assert_eq!(
            unspecified.checked_add(&torque).unwrap().kind,
            QuantityKind::Torque
        );
        assert!(
            q("1 Hz").checked_cmp(&q("1 Bq")).is_err(),
            "frequency is not activity"
        );
        assert_eq!(q("1 Hz").checked_cmp(&q("1 s^-1")), Ok(Ordering::Equal));
        assert_eq!(
            Quantity::parse("5 J torque"),
            Err(QuantityError::KindMismatch {
                left: QuantityKind::Energy,
                right: QuantityKind::Torque
            }),
            "the joule declares energy"
        );
        assert!(matches!(
            Quantity::parse("3 kg torque"),
            Err(QuantityError::KindDimensionMismatch { .. })
        ));
        assert_eq!(
            Quantity::parse("3 kg heavy"),
            Err(QuantityError::NotAQuantity)
        );
        let rotated = torque.checked_mul(&q("2 rad")).unwrap();
        assert_eq!(
            rotated.kind,
            QuantityKind::Unspecified,
            "a product declares no kind"
        );
    }

    /// G124: exact interval arithmetic -- the derived interval of every operation contains the
    /// exact result for every operand drawn from the operands' intervals (checked at the bounds and
    /// the nominal values, where these monotone operations take their extremes).
    #[test]
    fn a_derived_interval_contains_every_exact_result() {
        let operands = [
            "12 ± 0.5 mm",
            "-3 ± 2 mm",
            "0.25 mm",
            "7 +/- 0.125 mm",
            "-0.5 ± 0.25 mm",
        ];
        let points = |x: &Quantity| {
            let (low, high) = x.bounds();
            [low, x.si_value, high]
        };
        let within = |value: Rational, result: &Quantity| {
            let (low, high) = result.bounds();
            low.checked_cmp(value) != Some(Ordering::Greater)
                && value.checked_cmp(high) != Some(Ordering::Greater)
        };
        for a in operands {
            for b in operands {
                let (x, y) = (q(a), q(b));
                type Op = fn(&Quantity, &Quantity) -> Result<Quantity, QuantityError>;
                type Exact = fn(Rational, Rational) -> Option<Rational>;
                let ops: [(Op, Exact); 4] = [
                    (Quantity::checked_add, Rational::checked_add),
                    (Quantity::checked_sub, |u, v| {
                        u.checked_add(v.checked_neg()?)
                    }),
                    (Quantity::checked_mul, Rational::checked_mul),
                    (Quantity::checked_div, Rational::checked_div),
                ];
                for (op, exact) in ops {
                    let Ok(result) = op(&x, &y) else {
                        assert!(
                            y.bounds().0.checked_cmp(Rational::ZERO) != Some(Ordering::Greater),
                            "{a} op {b}: only a divisor interval reaching zero is refused"
                        );
                        continue;
                    };
                    assert!(within(result.si_value, &result), "{a} op {b}: nominal");
                    for u in points(&x) {
                        for v in points(&y) {
                            let value = exact(u, v).unwrap();
                            assert!(within(value, &result), "{a} op {b}: {value} escapes");
                        }
                    }
                }
            }
        }
        let widened = q("12 ± 0.5 mm").checked_add(&q("3 ± 0.25 mm")).unwrap();
        assert_eq!(
            widened.bounds(),
            (q("14.25 mm").si_value, q("15.75 mm").si_value)
        );
        assert_eq!(
            q("2 mm").checked_add(&q("3 mm")).unwrap().uncertainty,
            None,
            "exact stays exact"
        );
    }

    #[test]
    fn overlapping_intervals_are_undecided_never_guessed() {
        assert_eq!(
            q("10 ± 1 mm").checked_cmp(&q("10.5 mm")),
            Err(QuantityError::Undecided)
        );
        assert_eq!(q("10 ± 1 mm").checked_cmp(&q("12 mm")), Ok(Ordering::Less));
        assert_eq!(
            q("10 ± 1 mm").checked_cmp(&q("8.5 ± 0.25 mm")),
            Ok(Ordering::Greater)
        );
        assert_eq!(
            q("1 m").checked_div(&q("0 ± 1 m")),
            Err(QuantityError::UncertainDivisor)
        );
        assert_eq!(
            Quantity::parse("1 ± -1 m"),
            Err(QuantityError::InvalidUncertainty)
        );
        assert!(Quantity::is_quantity_shaped("120 ± 0.5 mm"));
        assert!(Quantity::is_quantity_shaped("5 N*m torque"));
        assert!(!Quantity::is_quantity_shaped("± 5 mm"));
    }

    #[test]
    fn leaving_exact_arithmetic_is_marked() {
        assert!(q("0.5 m").approximate().exact);
        assert!(q("3 m").approximate().exact);
        assert!(!q("0.1 m").approximate().exact, "1/10 has no exact f64");
        assert_eq!(q("0.1 m").approximate().value, 0.1);
    }

    #[test]
    fn equal_quantities_in_different_units_are_exactly_equal() {
        for (a, b) in [
            ("0.12 m", "120 mm"),
            ("1 kg", "1000 g"),
            ("3.3 V", "3300 mV"),
            ("1 kΩ", "1000 ohm"),
            ("100 nF", "0.1 µF"),
            ("1 h", "3600 s"),
            ("2 min", "120 s"),
            ("1 kN", "1000 kg*m/s^2"),
            ("1 W", "1 V*A"),
            ("1 L", "0.001 m^3"),
            ("250 mL", "0.25 L"),
            ("1e-3 m", "1 mm"),
        ] {
            assert_eq!(q(a), q(b), "{a} == {b}");
        }
        // Exactness: 0.1 + 0.2 == 0.3, which binary floating point gets wrong.
        assert_eq!(q("0.1 m").checked_add(&q("0.2 m")).unwrap(), q("0.3 m"));
    }

    #[test]
    fn adding_different_dimensions_is_an_error_never_a_value() {
        let err = q("3 kg").checked_add(&q("2 V")).unwrap_err();
        assert!(
            matches!(err, QuantityError::DimensionMismatch { .. }),
            "{err}"
        );
        assert!(q("1 m").checked_cmp(&q("1 s")).is_err());
        assert!(
            q("1 rad").checked_add(&q("1 m/m")).is_err(),
            "angle is its own base"
        );
    }

    #[test]
    fn multiplication_and_division_combine_dimensions() {
        let power = q("12 V").checked_mul(&q("2 A")).unwrap();
        assert_eq!(power, q("24 W"));
        let resistance = q("5 V").checked_div(&q("10 mA")).unwrap();
        assert_eq!(resistance, q("500 ohm"));
        let speed = q("10 m").checked_div(&q("2 s")).unwrap();
        assert_eq!(
            speed.dimension,
            Dimension {
                exponents: [1, 0, -1, 0, 0, 0, 0, 0]
            }
        );
        assert_eq!(
            q("1 m").checked_div(&q("1 m")).unwrap().dimension,
            Dimension::DIMENSIONLESS
        );
    }

    #[test]
    fn ordering_is_exact_across_units() {
        assert_eq!(
            q("2 mm").checked_cmp(&q("0.002 m")).unwrap(),
            Ordering::Equal
        );
        assert_eq!(q("1.9 mm").checked_cmp(&q("2 mm")).unwrap(), Ordering::Less);
        assert_eq!(
            q("1 kg").checked_cmp(&q("999 g")).unwrap(),
            Ordering::Greater
        );
        assert_eq!(q("-3 V").checked_cmp(&q("2 V")).unwrap(), Ordering::Less);
    }

    #[test]
    fn inexact_or_affine_units_are_unsupported_never_approximated() {
        for text in [
            "90 deg",
            "25 degC",
            "3000 rpm",
            "5 furlong",
            "2 kmin",
            "1 kh",
        ] {
            assert!(
                matches!(
                    Quantity::parse(text),
                    Err(QuantityError::UnsupportedUnit { .. })
                ),
                "{text}"
            );
            assert!(Quantity::is_quantity_shaped(text), "{text}");
        }
    }

    #[test]
    fn non_quantities_are_recognized_as_such() {
        for text in [
            "backend", "rust", "1", "3.3", "", "V 3", "1.2.3 m", "12 m s",
        ] {
            assert!(Quantity::parse(text).is_err(), "{text}");
        }
        for text in ["backend", "rust", "1", "3.3", "", "apps/ui"] {
            assert!(!Quantity::is_quantity_shaped(text), "{text}");
        }
    }

    #[test]
    fn decimal_parsing_is_exact_and_bounded() {
        assert_eq!(Rational::parse_decimal("0.125"), Rational::new(1, 8));
        assert_eq!(
            Rational::parse_decimal("-1.5e2"),
            Some(Rational::integer(-150))
        );
        assert_eq!(Rational::parse_decimal("2.5E-3"), Rational::new(1, 400));
        assert_eq!(Rational::parse_decimal(".5"), Rational::new(1, 2));
        for bad in ["", "-", "e3", "1e", "1..2", "1e999", "abc"] {
            assert_eq!(Rational::parse_decimal(bad), None, "{bad}");
        }
        let huge = format!("1{} m", "0".repeat(60));
        assert!(
            Quantity::parse(&huge).is_err(),
            "overflow is reported, not wrapped"
        );
    }

    #[test]
    fn dimension_display_names_the_base_exponents() {
        assert_eq!(q("1 N").dimension.to_string(), "m*kg*s^-2");
        assert_eq!(Dimension::DIMENSIONLESS.to_string(), "1");
    }

    /// Algebraic oracle: for random exact values, (a + b) - b == a and (a * b) / b == a, and
    /// adding in any unit spelling of the same dimension gives the same SI value.
    #[test]
    fn exact_arithmetic_round_trips() {
        let mut state = 0x9E37_79B9_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % 20_000) as i128 - 10_000
        };
        for _ in 0..500 {
            let a = Quantity::new(
                Rational::new(next(), 1 + next().abs()).unwrap(),
                q("1 V").dimension,
            );
            let b = Quantity::new(
                Rational::new(next(), 1 + next().abs()).unwrap(),
                q("1 V").dimension,
            );
            assert_eq!(a.checked_add(&b).unwrap().checked_sub(&b).unwrap(), a);
            if b.si_value != Rational::ZERO {
                let product = a.checked_mul(&b).unwrap();
                assert_eq!(product.checked_div(&b).unwrap(), a);
            }
        }
    }
}
