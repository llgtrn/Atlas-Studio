use super::*;

fn bytes<const N: usize>(hex: &str) -> [u8; N] {
    let raw: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    raw.try_into().unwrap()
}

fn message(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

/// RFC 8032 section 7.1, TEST 1-3: (public key, message, signature). TESTs 2 and 3 were
/// reproduced by OpenSSL 3.0 from their secret keys; all three by the differential oracle below.
const RFC_8032: [(&str, &str, &str); 3] = [
    (
        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
        "",
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
    ),
    (
        "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
        "72",
        "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
    ),
    (
        "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025",
        "af82",
        "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a",
    ),
];

#[test]
fn curve_constants_are_the_published_ones() {
    // Encodings computed independently with Python big integers.
    let hex = |b: [u8; 32]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    assert_eq!(
        hex(curve_d().to_bytes()),
        "a3785913ca4deb75abd841414d0a700098e879777940c78c73fe6f2bee6c0352"
    );
    assert_eq!(
        hex(sqrt_minus_one().to_bytes()),
        "b0a00e4a271beec478e42fad0618432fa7d7fb3d99004d2b0bdfc14f8024832b"
    );
    assert!(sqrt_minus_one().square().equals(Fe::ONE.neg()));
    let base = Point::decompress(&BASE).unwrap();
    assert_eq!(base.compress(), BASE);
    // The base point has order L.
    assert!(base.times(&L).is_identity());
    assert!(!base.is_small_order());
}

#[test]
fn field_encoding_is_canonical() {
    // p itself and p + 1 are non-canonical encodings of 0 and 1.
    let p: [u8; 32] = bytes("edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f");
    assert_eq!(Fe::from_bytes(&p).to_bytes(), [0; 32]);
    let mut p1 = p;
    p1[0] += 1;
    assert_eq!(Fe::from_bytes(&p1).to_bytes()[0], 1);
    // A point whose y is written as y + p is refused.
    let mut y_plus_p = [0u8; 32];
    y_plus_p.copy_from_slice(&p1);
    assert!(Point::decompress(&y_plus_p).is_none());
}

#[test]
fn scalars_reduce_mod_l() {
    let mut wide = [0u8; 64];
    wide[..32].copy_from_slice(&L);
    assert_eq!(reduce_wide(&wide), [0; 32]);
    wide[0] += 5;
    let mut five = [0u8; 32];
    five[0] = 5;
    assert_eq!(reduce_wide(&wide), five);
    // (2^512 - 1) mod L, computed with Python big integers.
    let expected: [u8; 32] =
        bytes("000f9c44e31106a447938568a71b0ed065bef517d273ecce3d9a307c1b419903");
    assert_eq!(reduce_wide(&[0xff; 64]), expected);
    assert!(!is_reduced(&L));
    let mut below = L;
    below[0] -= 1;
    assert!(is_reduced(&below));
}

#[test]
fn rfc_8032_vectors_verify() {
    for (key, msg, sig) in RFC_8032 {
        assert!(
            verify(&bytes(key), &message(msg), &bytes(sig)),
            "RFC 8032 vector with message {msg:?}"
        );
    }
}

#[test]
fn any_change_to_key_message_or_signature_fails() {
    let (key, msg, sig) = RFC_8032[2];
    let (key, msg, sig): ([u8; 32], Vec<u8>, [u8; 64]) = (bytes(key), message(msg), bytes(sig));
    for bit in (0..256).step_by(5) {
        let mut k = key;
        k[bit / 8] ^= 1 << (bit % 8);
        assert!(!verify(&k, &msg, &sig), "key bit {bit}");
    }
    for bit in 0..msg.len() * 8 {
        let mut m = msg.clone();
        m[bit / 8] ^= 1 << (bit % 8);
        assert!(!verify(&key, &m, &sig), "message bit {bit}");
    }
    let mut longer = msg.clone();
    longer.push(0);
    assert!(!verify(&key, &longer, &sig));
    for bit in (0..512).step_by(7) {
        let mut s = sig;
        s[bit / 8] ^= 1 << (bit % 8);
        assert!(!verify(&key, &msg, &s), "signature bit {bit}");
    }
}

#[test]
fn malleated_and_small_order_inputs_are_refused() {
    let (key, msg, sig) = RFC_8032[1];
    let (key, msg, sig): ([u8; 32], Vec<u8>, [u8; 64]) = (bytes(key), message(msg), bytes(sig));
    // S + L verifies under the bare group equation, but S must be reduced.
    let mut malleated = sig;
    let mut carry = 0u16;
    for i in 0..32 {
        let sum = sig[32 + i] as u16 + L[i] as u16 + carry;
        malleated[32 + i] = sum as u8;
        carry = sum >> 8;
    }
    assert_eq!(carry, 0);
    assert!(!verify(&key, &msg, &malleated));
    // The identity (a small-order point) as a public key or as R.
    let mut identity = [0u8; 32];
    identity[0] = 1;
    assert!(Point::decompress(&identity).unwrap().is_small_order());
    assert!(!verify(&identity, &msg, &sig));
    let mut small_r = sig;
    small_r[..32].copy_from_slice(&identity);
    assert!(!verify(&key, &msg, &small_r));
    // The universal forgery small-order points allow: A = R = identity and S = 0 satisfy the
    // group equation for every message, so only the small-order refusal stops it.
    let mut forgery = [0u8; 64];
    forgery[..32].copy_from_slice(&identity);
    let base = Point::decompress(&BASE).unwrap();
    let r = Point::decompress(&identity).unwrap();
    assert_eq!(
        base.times(&[0; 32]).compress(),
        r.add(r.times(&[0x11; 32])).compress(),
        "the equation holds for the forgery"
    );
    for msg in [b"select".as_slice(), b"anything else".as_slice()] {
        assert!(!verify(&identity, msg, &forgery));
    }
    // Each refusal alone: a small-order key with an honest-looking R, and a small-order R under
    // an honest key, each satisfying the group equation for its message.
    let mut s = [0u8; 32];
    s[0] = 5;
    let mut key_only = [0u8; 64];
    key_only[..32].copy_from_slice(&base.times(&s).compress());
    key_only[32..].copy_from_slice(&s);
    assert!(!verify(&identity, b"select", &key_only));
    let mut secret = [0u8; 32];
    secret[0] = 7;
    let honest = base.times(&secret).compress();
    let k = reduce_wide(&sha512::hash_parts(&[&identity, &honest, b"select"]));
    let mut product = [0u8; 64];
    for (i, x) in k.iter().enumerate() {
        let mut carry = 0u32;
        for (j, y) in secret.iter().enumerate() {
            let sum = product[i + j] as u32 + (*x as u32) * (*y as u32) + carry;
            product[i + j] = sum as u8;
            carry = sum >> 8;
        }
        product[i + 32] = carry as u8;
    }
    let mut r_only = [0u8; 64];
    r_only[..32].copy_from_slice(&identity);
    r_only[32..].copy_from_slice(&reduce_wide(&product));
    assert!(
        base.times(&r_only[32..].try_into().unwrap()).compress()
            == r.add(Point::decompress(&honest).unwrap().times(&k))
                .compress(),
        "the equation holds for the small-order R"
    );
    assert!(!verify(&honest, b"select", &r_only));
    // A y with no point on the curve (y = 2 has no x).
    let mut off_curve = [0u8; 32];
    off_curve[0] = 2;
    assert!(Point::decompress(&off_curve).is_none());
    assert!(!verify(&off_curve, &msg, &sig));
}

/// Differential: the independent `ed25519-compact` (test-only, no dependencies) signs; Atlas
/// verifies every valid signature and agrees with its verification on tampered ones.
#[test]
fn verification_agrees_with_an_independent_implementation() {
    use ed25519_compact::{KeyPair, PublicKey, Seed, Signature};
    for case in 0u32..24 {
        let seed = sha512::hash(&case.to_le_bytes());
        let pair = KeyPair::from_seed(Seed::new(seed[..32].try_into().unwrap()));
        let msg: Vec<u8> = (0..(case as usize * 37) % 300)
            .map(|i| (i as u32 * 31 + case) as u8)
            .collect();
        let signature: [u8; 64] = *pair.sk.sign(&msg, None);
        let key: [u8; 32] = *pair.pk;
        assert!(verify(&key, &msg, &signature), "case {case}");
        let mut tampered = signature;
        tampered[(case as usize * 11) % 64] ^= 1 << (case % 8);
        let expected = PublicKey::new(key)
            .verify(&msg, &Signature::new(tampered))
            .is_ok();
        assert_eq!(
            verify(&key, &msg, &tampered),
            expected,
            "tampered case {case}"
        );
    }
    // RFC 8032 TEST 1 from its secret key, through the oracle.
    let pair = KeyPair::from_seed(Seed::new(bytes(
        "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
    )));
    let (key, _, sig) = RFC_8032[0];
    assert_eq!(*pair.pk, bytes::<32>(key));
    assert_eq!(*pair.sk.sign(b"", None), bytes::<64>(sig));
}
