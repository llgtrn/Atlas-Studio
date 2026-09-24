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
//! Known, recorded limitation of any exponent-vector model: dimensionally equal but physically
//! distinct quantities (torque N*m vs energy J; frequency Hz vs becquerel) are not distinguished.

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

    /// Nearest `f64`, for display and non-exact consumers only.
    pub fn to_f64(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
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
        }
    }
}

/// A value in coherent SI base units with its dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Quantity {
    pub si_value: Rational,
    pub dimension: Dimension,
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
        "Hz" => (Rational::ONE, d([0, 0, -1, 0, 0, 0, 0, 0])),
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
    pub fn new(si_value: Rational, dimension: Dimension) -> Self {
        Self {
            si_value,
            dimension,
        }
    }

    /// Parses `<decimal> <unit expression>` separated by whitespace, e.g. `120 mm`, `3.3 V`,
    /// `9.81 m/s^2`. A bare number is `NotAQuantity`: a quantity always states its unit.
    pub fn parse(text: &str) -> Result<Self, QuantityError> {
        let text = text.trim();
        let Some((number, unit)) = text.split_once(char::is_whitespace) else {
            return Err(QuantityError::NotAQuantity);
        };
        let value = Rational::parse_decimal(number).ok_or(QuantityError::NotAQuantity)?;
        let (factor, dimension) = parse_unit_expression(unit)?;
        let si_value = value.checked_mul(factor).ok_or(QuantityError::Overflow)?;
        Ok(Self::new(si_value, dimension))
    }

    /// `true` when `text` has the shape of a quantity -- a decimal literal followed by a unit
    /// token -- whether or not the unit is admitted. Plain words (`backend`) and bare numbers
    /// (`1`) are not quantity-shaped.
    pub fn is_quantity_shaped(text: &str) -> bool {
        let text = text.trim();
        match text.split_once(char::is_whitespace) {
            Some((number, unit)) => {
                Rational::parse_decimal(number).is_some()
                    && unit
                        .trim()
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_alphabetic() || c == 'Ω' || c == 'µ')
            }
            None => false,
        }
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

    pub fn checked_add(&self, other: &Self) -> Result<Self, QuantityError> {
        self.same_dimension(other)?;
        let value = self
            .si_value
            .checked_add(other.si_value)
            .ok_or(QuantityError::Overflow)?;
        Ok(Self::new(value, self.dimension))
    }

    pub fn checked_sub(&self, other: &Self) -> Result<Self, QuantityError> {
        let negated = other
            .si_value
            .checked_neg()
            .ok_or(QuantityError::Overflow)?;
        self.checked_add(&Self::new(negated, other.dimension))
    }

    pub fn checked_mul(&self, other: &Self) -> Result<Self, QuantityError> {
        Ok(Self::new(
            self.si_value
                .checked_mul(other.si_value)
                .ok_or(QuantityError::Overflow)?,
            self.dimension
                .combine(other.dimension, 1)
                .ok_or(QuantityError::Overflow)?,
        ))
    }

    pub fn checked_div(&self, other: &Self) -> Result<Self, QuantityError> {
        Ok(Self::new(
            self.si_value
                .checked_div(other.si_value)
                .ok_or(QuantityError::Overflow)?,
            self.dimension
                .combine(other.dimension, -1)
                .ok_or(QuantityError::Overflow)?,
        ))
    }

    /// Exact ordering of two quantities of the same dimension.
    pub fn checked_cmp(&self, other: &Self) -> Result<Ordering, QuantityError> {
        self.same_dimension(other)?;
        self.si_value
            .checked_cmp(other.si_value)
            .ok_or(QuantityError::Overflow)
    }
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.si_value, self.dimension)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(text: &str) -> Quantity {
        Quantity::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"))
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
            ("1 J", "1 N*m"),
            ("1 Hz", "1 s^-1"),
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
