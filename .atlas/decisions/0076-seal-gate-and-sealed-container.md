---
id: atlas.decision.0076.seal-gate-and-sealed-container
type: decision
status: accepted
canonical: true
---
# ADR 0076 — The seal gate and the SEALED census container (G161)

## Context

DEBT-SEAL_GATE has four milestones:
- M3, the scoped certificate policy;
- M4, the SealPolicy record;
- M8, the eligibility gate;
- M9, the SEALED encoder and reader.

G149 (ADR 0065) delivered M3 and M4. `seal evaluate` applies the declared policy to a certificate's blockers. Nothing else could decide a seal:
- The `.atlas` writer refused every seal status except `UNSEALED_CENSUS_CONTAINER`.
- The reader rejected every container that was not unsealed.

G127 (ADR 0060) produces the verification report, and G138 (ADR 0053) produces the integrity report. Until this generation nothing consumed either of them, and DEBT-VERIFICATION_EVIDENCE names that as an open gap. G148 made a SelectedDesign a typed record, but no seal required one.

## Decision

1. **A seal is decided in one place: `atlas_core::seal::gate` (M8).** It joins five inputs on one candidate, the census container being sealed. It decides ELIGIBLE only when every input admits exactly that candidate:
   - **The policy.** It must be valid, and its identity must verify.
   - **The certificate.** Its verdict is recomputed from the certificate's own blockers under the policy; a stored verdict is never read. The certificate must be the container's own and at least CLOSED.
   - **The verification report.** It must be ADMISSIBLE, under the policy's verification classes, and its candidate must equal the container's census digest.
   - **The integrity report.** It must carry the verdict the policy requires. It must name the container's revision, and its observed architecture root must equal the census digest.
   - **The design.** It must be SELECTED by an authority event and rest on a comparison. Its identity must verify, and its parent root, candidate and scope must be this container's.

   A NOT_ELIGIBLE decision carries every reason, each typed as an `IneligibleReason`. An ELIGIBLE decision carries a `SealRecord`. The record names:
   - the policy;
   - the certificate, census digest and revision;
   - the verification report's identity;
   - the integrity report and its envelope;
   - the design.

   It also lists every certificate blocker the policy permitted, with the reason. The record's identity is its own digest.
2. **Revisions are joined by name, and a dirty tree has none.**
   - A container records its revision as `git:<sha>`, with `+dirty` appended for a dirty tree. An integrity report names `revision:<sha>`.
   - The gate maps a clean `git:<sha>` to `revision:<sha>` and joins nothing else.
   - The first wiring of the gate compared the strings directly, so it would have refused every real container. The runtime fixture exposed that before any evidence was written.
3. **The SEALED container (M9).** The wire gains:
   - header flag bit 1, `SEALED`;
   - the manifest status `SEALED_CENSUS_CONTAINER`;
   - section 18, `seal-record`. Its fields are inline UTF-8 text: the record, then one `permitted` record per permitted blocker.

   The flags must be exactly UNSEALED or exactly SEALED, and must agree with the manifest status. The SEAL section is present exactly when the container is sealed. A new section type needs a new minor (G97), so a sealed container is format minor 2. An unsealed container stays minor 1, byte for byte what a minor-1 reader reads.
4. **The writer refuses, and the reader rejects, a seal that does not bind its container.** `check_record` runs on both sides. It requires:
   - the record's schema;
   - an identity that verifies;
   - the container's certificate, census digest and revision;
   - a `permitted` entry for every blocker of the container's certificate. A blocker added after the seal was decided is therefore unsealed.

   The reader cannot re-run the gate, because the reports are not inside the container. It verifies what the container can prove: that the record is the gate's shape and binds this container.
5. **A sealed root has no undeclared bytes.** The wire contract says that "a reader verifying a SEALED root MUST reject" fields its schema does not declare. Unsealed readers skip unknown optional fields. A sealed container must be exactly its own canonical encoding under the schema identities it carries, so an undeclared field anywhere, including inside the seal section, is rejected.
6. **Runtime and CLI.**
   - `runtime::seal::gate_container` reads the verified container. The binding and certificate come from the container, never from a separate file. It then reads the two reports and an optional design.
   - `seal_container` writes the sealed container only from an ELIGIBLE decision. It refuses a container that is already sealed.
   - `atlas-systemizer seal gate` exits SEAL_NOT_ELIGIBLE with every reason.
   - `atlas-systemizer atlas seal` writes the sealed container.

## Consequences

- **DEBT-SEAL_GATE:** M3, M4, M8 and M9 are delivered. The self scope is still not sealed. Under its own policy the Atlas self-scope container is NOT_ELIGIBLE, and the clean-HEAD evidence records `seal gate` naming why:
  - the certificate is RECONCILED, not CLOSED;
  - its refused blockers;
  - there is no SELECTED design.

  No principal is declared, and a coding agent cannot select a design. MIN_ATLASX remains open.
- **DEBT-VERIFICATION_EVIDENCE:** the verification and integrity reports have their first consumer. The gate reads their verdicts and candidates.
- **Not decided here:**
  - re-certifying after a seal, when a certificate becomes SEALED;
  - sealing sharded or FAT containers;
  - accepting a sealed container written under an older schema generation. A conforming generation still encodes to its own bytes, and a non-conforming one is refused by schema identity as before.
