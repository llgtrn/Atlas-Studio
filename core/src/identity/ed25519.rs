//! Ed25519 signature verification (RFC 8032, section 5.1.7), natively owned by Atlas (G171,
//! ADR 0085).
//!
//! Verification only. Every input is public -- a declared public key, a message and a signature --
//! so this module has no secret to protect and runs in variable time. Signing takes a secret key
//! and is deliberately absent: a principal signs outside Atlas with its own tool, and Atlas never
//! holds a private key (the ADR 0005 boundary for secret inputs).
//!
//! Verification is strict and cofactorless, refusing rather than guessing:
//! - a public key or `R` that is not a canonical point encoding, or is of small order, fails;
//! - `S` must be reduced (`S < L`);
//! - the check is `[S]B = R + [k]A`, compared by canonical encodings, with
//!   `k = SHA-512(R || A || M) mod L`.
//!
//! Pinned by RFC 8032's test vectors and differentially checked against an independent
//! implementation in tests.

use super::sha512;

/// A field element of GF(2^255 - 19) in five 51-bit limbs.
#[derive(Clone, Copy, Debug)]
struct Fe([u64; 5]);

const MASK: u64 = (1 << 51) - 1;

/// Little-endian exponents: p - 2 (inversion), (p - 5) / 8 (square roots), (p - 1) / 4.
const P_MINUS_2: [u8; 32] = exponent(0xeb, 0x7f);
const P_MINUS_5_OVER_8: [u8; 32] = exponent(0xfd, 0x0f);
const P_MINUS_1_OVER_4: [u8; 32] = exponent(0xfb, 0x1f);

const fn exponent(low: u8, high: u8) -> [u8; 32] {
    let mut bytes = [0xff; 32];
    bytes[0] = low;
    bytes[31] = high;
    bytes
}

/// The group order L = 2^252 + 27742317777372353535851937790883648493, little-endian.
const L: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
];

/// The base point's encoding: y = 4/5 with x even.
const BASE: [u8; 32] = {
    let mut bytes = [0x66; 32];
    bytes[0] = 0x58;
    bytes
};

impl Fe {
    const ZERO: Fe = Fe([0; 5]);
    const ONE: Fe = Fe([1, 0, 0, 0, 0]);

    fn from_u64(value: u64) -> Fe {
        Fe([value & MASK, value >> 51, 0, 0, 0])
    }

    /// The low 255 bits of `bytes` (the top bit is ignored).
    fn from_bytes(bytes: &[u8; 32]) -> Fe {
        let load = |i: usize| u64::from_le_bytes(bytes[i..i + 8].try_into().unwrap());
        Fe([
            load(0) & MASK,
            (load(6) >> 3) & MASK,
            (load(12) >> 6) & MASK,
            (load(19) >> 1) & MASK,
            (load(24) >> 12) & MASK,
        ])
    }

    fn carried(mut h: [u64; 5]) -> Fe {
        for _ in 0..2 {
            for i in 0..4 {
                h[i + 1] += h[i] >> 51;
                h[i] &= MASK;
            }
            h[0] += (h[4] >> 51) * 19;
            h[4] &= MASK;
        }
        Fe(h)
    }

    /// The canonical little-endian encoding (fully reduced below p).
    fn to_bytes(self) -> [u8; 32] {
        let mut h = Fe::carried(self.0).0;
        let mut q = (h[0] + 19) >> 51;
        for limb in &h[1..] {
            q = (limb + q) >> 51;
        }
        h[0] += 19 * q;
        for i in 0..4 {
            h[i + 1] += h[i] >> 51;
            h[i] &= MASK;
        }
        h[4] &= MASK;
        let mut out = [0u8; 32];
        let (mut acc, mut bits, mut index) = (0u128, 0, 0);
        for limb in h {
            acc |= (limb as u128) << bits;
            bits += 51;
            while bits >= 8 {
                out[index] = acc as u8;
                acc >>= 8;
                bits -= 8;
                index += 1;
            }
        }
        out[index] = acc as u8;
        out
    }

    fn add(self, other: Fe) -> Fe {
        let mut h = self.0;
        for (limb, o) in h.iter_mut().zip(other.0) {
            *limb += o;
        }
        Fe::carried(h)
    }

    /// `self - other`, as `self + 4p - other` so no limb underflows.
    fn sub(self, other: Fe) -> Fe {
        let four_p = [(MASK - 18) * 4, MASK * 4, MASK * 4, MASK * 4, MASK * 4];
        let a = Fe::carried(self.0).0;
        let b = Fe::carried(other.0).0;
        let mut h = [0u64; 5];
        for i in 0..5 {
            h[i] = a[i] + four_p[i] - b[i];
        }
        Fe::carried(h)
    }

    fn neg(self) -> Fe {
        Fe::ZERO.sub(self)
    }

    fn mul(self, other: Fe) -> Fe {
        let a = Fe::carried(self.0).0;
        let b = Fe::carried(other.0).0;
        let m = |x: u64, y: u64| x as u128 * y as u128;
        let b19 = [0, b[1] * 19, b[2] * 19, b[3] * 19, b[4] * 19];
        let mut r = [
            m(a[0], b[0]) + m(a[1], b19[4]) + m(a[2], b19[3]) + m(a[3], b19[2]) + m(a[4], b19[1]),
            m(a[0], b[1]) + m(a[1], b[0]) + m(a[2], b19[4]) + m(a[3], b19[3]) + m(a[4], b19[2]),
            m(a[0], b[2]) + m(a[1], b[1]) + m(a[2], b[0]) + m(a[3], b19[4]) + m(a[4], b19[3]),
            m(a[0], b[3]) + m(a[1], b[2]) + m(a[2], b[1]) + m(a[3], b[0]) + m(a[4], b19[4]),
            m(a[0], b[4]) + m(a[1], b[3]) + m(a[2], b[2]) + m(a[3], b[1]) + m(a[4], b[0]),
        ];
        for i in 0..4 {
            r[i + 1] += r[i] >> 51;
            r[i] &= MASK as u128;
        }
        r[0] += (r[4] >> 51) * 19;
        r[4] &= MASK as u128;
        // r[0] may now exceed 64 bits: carry it once more before narrowing.
        r[1] += r[0] >> 51;
        r[0] &= MASK as u128;
        Fe::carried(r.map(|limb| limb as u64))
    }

    fn square(self) -> Fe {
        self.mul(self)
    }

    /// `self` raised to a little-endian exponent, square-and-multiply (public inputs only).
    fn pow(self, exponent: &[u8; 32]) -> Fe {
        let mut result = Fe::ONE;
        for bit in (0..256).rev() {
            result = result.square();
            if (exponent[bit / 8] >> (bit % 8)) & 1 == 1 {
                result = result.mul(self);
            }
        }
        result
    }

    fn invert(self) -> Fe {
        self.pow(&P_MINUS_2)
    }

    fn equals(self, other: Fe) -> bool {
        self.to_bytes() == other.to_bytes()
    }

    fn is_zero(self) -> bool {
        self.to_bytes() == [0; 32]
    }

    fn is_negative(self) -> bool {
        self.to_bytes()[0] & 1 == 1
    }
}

/// d = -121665 / 121666.
fn curve_d() -> Fe {
    static D: std::sync::OnceLock<Fe> = std::sync::OnceLock::new();
    *D.get_or_init(|| {
        Fe::from_u64(121665)
            .neg()
            .mul(Fe::from_u64(121666).invert())
    })
}

/// A square root of -1: 2^((p - 1) / 4).
fn sqrt_minus_one() -> Fe {
    static SQRT_M1: std::sync::OnceLock<Fe> = std::sync::OnceLock::new();
    *SQRT_M1.get_or_init(|| Fe::from_u64(2).pow(&P_MINUS_1_OVER_4))
}

/// A point in extended twisted Edwards coordinates: x = X/Z, y = Y/Z, T = XY/Z.
#[derive(Clone, Copy, Debug)]
struct Point {
    x: Fe,
    y: Fe,
    z: Fe,
    t: Fe,
}

impl Point {
    const IDENTITY: Point = Point {
        x: Fe::ZERO,
        y: Fe::ONE,
        z: Fe::ONE,
        t: Fe::ZERO,
    };

    /// The point a canonical encoding names, or `None` for a non-canonical `y`, a `y` with no
    /// point on the curve, or the negative zero `x`.
    fn decompress(bytes: &[u8; 32]) -> Option<Point> {
        let sign = bytes[31] >> 7 == 1;
        let mut y_bytes = *bytes;
        y_bytes[31] &= 0x7f;
        let y = Fe::from_bytes(&y_bytes);
        if y.to_bytes() != y_bytes {
            return None;
        }
        let d = curve_d();
        let y2 = y.square();
        let u = y2.sub(Fe::ONE);
        let v = d.mul(y2).add(Fe::ONE);
        let v3 = v.square().mul(v);
        let v7 = v3.square().mul(v);
        let mut x = u.mul(v3).mul(u.mul(v7).pow(&P_MINUS_5_OVER_8));
        let vx2 = v.mul(x.square());
        if !vx2.equals(u) {
            if !vx2.equals(u.neg()) {
                return None;
            }
            x = x.mul(sqrt_minus_one());
        }
        if x.is_zero() && sign {
            return None;
        }
        if x.is_negative() != sign {
            x = x.neg();
        }
        Some(Point {
            x,
            y,
            z: Fe::ONE,
            t: x.mul(y),
        })
    }

    fn compress(self) -> [u8; 32] {
        let z_inverse = self.z.invert();
        let x = self.x.mul(z_inverse);
        let y = self.y.mul(z_inverse);
        let mut bytes = y.to_bytes();
        bytes[31] |= (x.is_negative() as u8) << 7;
        bytes
    }

    /// Unified addition (add-2008-hwcd-3, a = -1), complete on this curve, so it doubles too.
    fn add(self, other: Point) -> Point {
        let two_d = curve_d().add(curve_d());
        let a = self.y.sub(self.x).mul(other.y.sub(other.x));
        let b = self.y.add(self.x).mul(other.y.add(other.x));
        let c = self.t.mul(two_d).mul(other.t);
        let d = self.z.add(self.z).mul(other.z);
        let e = b.sub(a);
        let f = d.sub(c);
        let g = d.add(c);
        let h = b.add(a);
        Point {
            x: e.mul(f),
            y: g.mul(h),
            z: f.mul(g),
            t: e.mul(h),
        }
    }

    /// `[scalar] self` for a little-endian scalar, double-and-add (public inputs only).
    fn times(self, scalar: &[u8; 32]) -> Point {
        let mut result = Point::IDENTITY;
        for bit in (0..256).rev() {
            result = result.add(result);
            if (scalar[bit / 8] >> (bit % 8)) & 1 == 1 {
                result = result.add(self);
            }
        }
        result
    }

    fn is_identity(self) -> bool {
        self.x.is_zero() && self.y.equals(self.z)
    }

    /// Whether `[8] self` is the identity (the point lies in the torsion subgroup).
    fn is_small_order(self) -> bool {
        let mut p = self;
        for _ in 0..3 {
            p = p.add(p);
        }
        p.is_identity()
    }
}

/// Whether the little-endian `scalar` is below the group order.
fn is_reduced(scalar: &[u8; 32]) -> bool {
    for i in (0..32).rev() {
        if scalar[i] != L[i] {
            return scalar[i] < L[i];
        }
    }
    false
}

/// A 64-byte little-endian integer reduced mod L, by binary long division.
fn reduce_wide(wide: &[u8; 64]) -> [u8; 32] {
    let l: [u64; 5] = {
        let mut limbs = [0u64; 5];
        for (i, chunk) in L.chunks(8).enumerate() {
            limbs[i] = u64::from_le_bytes(chunk.try_into().unwrap());
        }
        limbs
    };
    let at_least_l = |r: &[u64; 5]| {
        for i in (0..5).rev() {
            if r[i] != l[i] {
                return r[i] > l[i];
            }
        }
        true
    };
    let mut r = [0u64; 5];
    for bit in (0..512).rev() {
        let incoming = ((wide[bit / 8] >> (bit % 8)) & 1) as u64;
        let mut carry = incoming;
        for limb in r.iter_mut() {
            let next = *limb >> 63;
            *limb = (*limb << 1) | carry;
            carry = next;
        }
        if at_least_l(&r) {
            let mut borrow = 0u64;
            for (limb, subtrahend) in r.iter_mut().zip(l) {
                let (value, b1) = limb.overflowing_sub(subtrahend);
                let (value, b2) = value.overflowing_sub(borrow);
                *limb = value;
                borrow = (b1 || b2) as u64;
            }
        }
    }
    let mut out = [0u8; 32];
    for (chunk, limb) in out.chunks_mut(8).zip(r) {
        chunk.copy_from_slice(&limb.to_le_bytes());
    }
    out
}

/// Whether `signature` is a valid Ed25519 signature of `message` under `public_key` (strict,
/// cofactorless verification; see the module documentation).
pub fn verify(public_key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    let Some(a) = Point::decompress(public_key) else {
        return false;
    };
    let r_bytes: [u8; 32] = signature[..32].try_into().unwrap();
    let s: [u8; 32] = signature[32..].try_into().unwrap();
    let Some(r) = Point::decompress(&r_bytes) else {
        return false;
    };
    if a.is_small_order() || r.is_small_order() || !is_reduced(&s) {
        return false;
    }
    let k = reduce_wide(&sha512::hash_parts(&[&r_bytes, public_key, message]));
    let base = Point::decompress(&BASE).expect("the base point decodes");
    base.times(&s).compress() == r.add(a.times(&k)).compress()
}

#[cfg(test)]
mod tests;
