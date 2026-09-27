---
id: atlas.decision.0085.authenticated-principals-and-gate-authority
type: decision
status: accepted
canonical: true
---
# ADR 0085 — Authenticated principals, and the seal gate asks for authority (G171)

## Context

G171 took NA-PRINCIPAL-AUTHENTICATION, the native queue head the pressure map selected after replay R11 (pressure 44, DEBT-IDENTITY_AUTHORITY and DEBT-SELECTED_DESIGN, each stale for 22 generations). Since G148 (ADR 0064), a selection needed an event from a principal named in `.atlas/declared/principals.json`. A name is a claim, not an authentication: anyone who can write a design file can write any declared principal's name into its event.

Attacking the debt surfaced a second, larger falsification. The seal gate (G161, ADR 0076) required only that a design's authority event *exist*: `runtime::seal::gate_container` read the design and never validated it against the registry. A SELECTED design whose event came from a PROVIDER, from an undeclared principal, or from an edited or forged event made the gate ELIGIBLE, as long as the other inputs admitted the candidate. The debt's falsification condition held: a selection accepted without authority.

No design has ever been SELECTED (the registry is empty), so no recorded seal rests on this hole.

## Decision

1. **Native Ed25519 verification, verification only.** `core::identity::sha512` implements FIPS 180-4 SHA-512, and `core::identity::ed25519` implements RFC 8032 verification. Verification is strict and cofactorless:
   - canonical point encodings only;
   - public keys and `R` of small order are refused;
   - `S < L`;
   - `[S]B = R + [k]A`, compared by encodings.

   Every input is public, so the implementation runs in variable time. Signing is deliberately absent: it takes a secret, and Atlas holds none (the ADR 0005 boundary). As with BLAKE3, an independent implementation (`ed25519-compact` 2.6.0, default features off, no dependencies) is a dev-dependency only, as a differential oracle. `ed25519-dalek` was tried first and refused by Atlas's own Cargo oracle test: its `curve25519-dalek` locks `fiat-crypto` behind a custom cfg that no host build downloads, so `cargo metadata --offline` could not run in a fresh environment.
2. **Keys in the registry, signatures on events.**
   - `PrincipalRegistry.keys` declares `{principal, algorithm: ed25519, public_key}`. A registry without keys still reads.
   - `AuthorityEvent.signature` carries `{public_key, signature}` over `signing_message(event)`, which is the event's recomputed identity under a signature domain. The signature is outside the identity, so the identity is fixed before it is signed, and any edit to a field the identity covers breaks the signature.
   - `design::validate` refuses a SELECTED event with any of these codes: AUTHORITY_EVENT_UNSIGNED, SIGNATURE_MALFORMED, SIGNING_KEY_UNDECLARED (the key is not declared for that principal, or not as `ed25519`), SIGNATURE_INVALID.
3. **One authority question, asked by the gate too.**
   - `design::authority_violations(design, registry)` answers whether an event carries selection authority: it selects this design under its mode, its identity verifies, its principal is declared and admitted, and it is signed under a declared key.
   - The seal gate takes the registry (`GateInputs.registry`; the CLI reads it from `--root`) and refuses a design whose event lacks authority with the new reason DESIGN_AUTHORITY_REFUSED, naming the codes.
4. **Atlas prepares and verifies; the principal signs.** `atlas-systemizer design event --design --principal --generation --statement` prints the unsigned event and the exact message to sign. The principal signs it with its own tool (for example `openssl pkeyutl -sign -rawin`) and pastes the lowercase-hex key and signature into the event. The AI proposes and never selects: no key is created, declared or used by Atlas on anyone's behalf, and the registry stays empty until the owner declares a principal and a key.

## Evidence

- **Primitives.**
  - SHA-512 matches the FIPS 180-4 examples and a ten-block input (expected digests from Python hashlib).
  - Ed25519 verifies RFC 8032 TESTs 1–3. OpenSSL 3.0 reproduced TESTs 2 and 3 from their secret keys, and `ed25519-compact` reproduces TEST 1.
  - The curve constants d and √−1, and a 512-bit reduction mod L, match Python big-integer values.
  - Tampering is refused: key bits, every message bit, signature bits, `S + L`, the identity as a key or `R`, and a `y` with no point.
  - A differential over 24 keys and messages agrees with `ed25519-compact` on valid and tampered signatures.
- **Interop.** An event prepared by `design event` and signed by OpenSSL, an independent signer, authenticates in Atlas and fails once a field changes (pinned in a core test).
- **Design rules.** A design test covers each code: unsigned, a stranger's valid signature, the declared key claimed with another key's signature, signed-then-edited, a flipped bit, uppercase hex, a key declared for another principal, and a key of another algorithm.
- **The gate.**
  - A falsifying gate test: a provider's event, an undeclared principal's, an unsigned one, a signed-then-edited one and one under an undeclared key were each ELIGIBLE before this change; each is now NOT_ELIGIBLE with DESIGN_AUTHORITY_REFUSED.
  - A runtime test runs gate, seal and read-back end to end with a signed fixture event, and refuses the unsigned event and the empty registry.
- **Mutants.** 8 killed:
  - the gate ignoring authority;
  - an undeclared key accepted;
  - any algorithm accepted;
  - an unsigned event accepted;
  - `S` not reduced;
  - small-order refusal removed, for both A and R together, for A alone and for R alone. A universal forgery (A = R = identity, S = 0) and one per point verify under the bare equation.

## Consequences

- DEBT-IDENTITY_AUTHORITY: principals are authenticated beyond the declared registry.
- DEBT-SELECTED_DESIGN: a selection, and a seal, now need an authenticated principal.
- **Still open:**
  - authority events bound to admissions (AdmissionTransaction does not exist as a record yet);
  - key rotation and revocation, and signing-time bounds;
  - the policy envelope for POLICY_AUTO;
  - DecisionProposal and ProviderReceipt lineage.
- **Still the owner's decision:** a SELECTED design needs the owner to declare a principal and a key, and to sign.
- **Not a semantic-extraction change.** Census records are unchanged, so no capability epoch moves and no replay verdict is affected.
