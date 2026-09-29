---
id: atlas.decision.0093.atlasx-precondition-gate
type: decision
status: accepted
canonical: true
---
# ADR 0093 — The AtlasX precondition gate (G179)

## Context

`ATLAS-TO-ATLASX.md` requires a precondition gate before any materialization: the parent must be a SEALED Atlas root, its seal must bind it, and the materialization must name the exact SelectedDesign and ArchitecturalIntegrityEnvelope the seal was decided on.

G161 (ADR 0076) built the seal gate (M8) and the SEALED container (M9). The integrity envelope (M5) and the SelectedDesign exist since G138 and G148. Nothing consumed a SEALED container, so construction node M10 (DEBT-ATLASX) was MISSING. ADR 0092 selected it as the G179 CREATION work.

## Decision

1. **One pure gate: `atlas_core::atlasx::precondition` (M10).** It takes three inputs as bytes: the parent container, the envelope (JSON) and, optionally, the design (JSON). It does no I/O. It admits the parent only when all of these hold:
   - the M9 reader verifies the parent, and the parent is SEALED;
   - the parent's seal record binds it. `seal::check_record` is re-run here rather than trusted to the reader;
   - the envelope's identity verifies, and it is the envelope the record names;
   - the design's identity verifies, it is SELECTED, and it is the design the record names, for this container's census and the sealed scope.

   `admit` runs the same checks on inputs that are already decoded.
2. **Typed verdicts.**
   - The verdict is ADMITTED or REFUSED (`PreconditionVerdict`).
   - ADMITTED carries the identities it bound: parent root, census digest, revision, scope, seal record, envelope and design.
   - REFUSED carries every reason found, sorted and deduplicated. Each reason is a `PreconditionRefusal`: `UNREADABLE`, `NOT_SEALED`, `SEAL_RECORD_UNBOUND`, `ENVELOPE_UNVERIFIED`, `ENVELOPE_MISMATCH`, `DESIGN_ABSENT`, `DESIGN_UNVERIFIED`, `DESIGN_MISMATCH` or `DESIGN_NOT_SELECTED`.
   - Input that does not decode is UNREADABLE, never a panic.
   - Both enums are in the vocabulary map.
3. **Every verdict lists what it does not verify** (`NOT_VERIFIED`), by the contract's step number:
   - Genome compatibility;
   - the seal policy, verification report and integrity report the record names;
   - the certificate and obligation state;
   - the design's selection authority;
   - root reachability;
   - materialization-critical obligations;
   - architectural impact.

   ADMITTED therefore means that the parent binds its seal, envelope and design. It does not mean that the whole contract precondition holds.
4. **Every input is explicit.** `atlas-systemizer atlasx precondition --atlas <container> --envelope <envelope.json> [--design <design.json>] [--out <path>]` prints the verdict as JSON. It exits `ATLASX_PRECONDITION_REFUSED` with the number of reasons. The envelope has no default: the contract forbids ambient inputs. A file that cannot be read is an error that names the file. `runtime::atlasx` reads the files. `runtime::atlasx::inputs` re-exports the types a caller builds the inputs from, so the CLI still reaches Core only through Runtime (a direct dev-dependency made AtlasCli -> Core an UNDECLARED Cargo edge in the self world model).

## Evidence

- **The G161 fixture in core** (`core/src/atlasx/precondition/tests.rs`). The fixture is sealed by the real seal gate.
  - The sealed container is ADMITTED with the identities it binds.
  - The same container unsealed is refused with exactly `NOT_SEALED`.
  - A seal record that is forged, re-stamped for another revision, or left open by a later blocker is refused with `SEAL_RECORD_UNBOUND`. As bytes, the M9 reader rejects such a record first, so it is `UNREADABLE`.
  - A record re-stamped to name another envelope or design reads back, and is refused with `ENVELOPE_MISMATCH` or `DESIGN_MISMATCH`.
  - A weakened envelope is refused with `ENVELOPE_UNVERIFIED`, and `ENVELOPE_MISMATCH` once re-stamped.
  - A widened design is refused with `DESIGN_UNVERIFIED`, and `DESIGN_MISMATCH` once re-stamped.
  - A superseded design is refused with `DESIGN_NOT_SELECTED`, and a missing one with `DESIGN_ABSENT`.
  - Every truncation of the sealed container is refused with `UNREADABLE`.
- **End to end through the CLI** (`apps/cli`). `seal gate` decides ELIGIBLE, `atlas seal` writes the container, and `atlasx precondition` admits it. The unsealed fixture exits `ATLASX_PRECONDITION_REFUSED` with `NOT_SEALED`.
- **The self scope.** The G178 self-scope container (root `blake3-256:724ce1f5…`) with the pinned envelope is REFUSED with `NOT_SEALED` and `DESIGN_ABSENT`: no SelectedDesign exists.
- **Mutation testing.** 32 mutants were run, and every one was killed:
  - 30 over the core checks: each check dropped and inverted, each clause of the design match dropped, and the verdict and order rules dropped;
  - 2 over the CLI exit code.

  One mutant, dropping DESIGN_ABSENT in the typed entry, first survived. It now has its test.

## Consequences

- **DEBT-ATLASX:** M10 is EXISTS. The materializer (M11), codec (M12) and validator (M13) remain MISSING, and they have an admitted-parent record to start from.
- **The self scope still cannot be materialized.** It is not sealed, because the certificate is RECONCILED and no design is SELECTED.
- **A SEALED container is self-certifying.** The seal record's identity is a digest, not a signature, and `atlas::write` accepts any record that binds the container. The precondition therefore does not prove that the seal gate decided the seal. Two things would close this: signing the seal, or re-running the gate over the named reports and the design's authority before materialization. That is recorded as a residual, not decided here. No SEALED container exists in the tree, and nothing materializes yet.
