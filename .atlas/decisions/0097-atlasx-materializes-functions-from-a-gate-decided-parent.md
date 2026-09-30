---
id: atlas.decision.0097.atlasx-materializes-functions-from-a-gate-decided-parent
type: decision
status: accepted
canonical: true
---
# ADR 0097 — The precondition re-runs the seal gate, and the materializer stages FUNCTIONS from a gate-decided parent (G185)

## Context

NA-ATLASX-MATERIALIZE (G185, CREATION, DEBT-ATLASX) asks for M11 then M13. It sets an order: first close RES-G179-SEAL-SELF-CERTIFYING, so that an admitted parent is one the seal gate decided, and then build the materializer over it.

The residual is real. A `SealRecord`'s identity is a digest of its own fields, not a signature. `atlas::write` accepts any record that binds the container's certificate, census and revision and permits its blockers. Anyone can take an unsealed census container and write a record with arbitrary policy and report ids. They set `seal_id = seal_identity(record)` and name a matching envelope and a SELECTED design. The container reads back SEALED, and the G179 precondition (ADR 0093) ADMITTED it, although no seal gate ran. ADR 0093 named two ways to close this: sign the seal, or re-run the gate over the named reports and the design's authority before materialization.

M12 (ADR 0095) writes and reads one AtlasX object class, FUNCTIONS. No object had ever been made from a parent Atlas.

## Decision

1. **The precondition re-runs the seal gate. The seal is not signed.** ADR 0085 rules out a seal signed by Atlas: Atlas never holds a private key. An owner-signed seal would add a second signed object, a second signing ceremony and a new key role, all for a decision the gate already makes deterministically from its inputs. The gate is a pure function. Its only authority input, the design's selection, is already signed by a declared principal and verified against the declared registry (G171). The precondition therefore judges a parent by two sets of inputs.
   - **What comes with the parent:** the container, the SelectedDesign, the verification report and the integrity report.
   - **The repository's declaration** (`Declaration`), never the parent's and never an operator's override: the declared seal policy, the pinned integrity envelope (`.atlas/declared/integrity-envelope.json`) and the declared principal registry. The runtime reads all three from `--root`. The CLI refuses `--policy` and `--envelope` on `atlasx precondition` and `atlasx materialize`.

   Beyond the G179 checks, it requires the following.
   - **The envelope is the pinned one.** The seal record must name it (`ENVELOPE_MISMATCH`: "the seal names envelope X, the repository pins Y"), and its identity must verify (`ENVELOPE_UNVERIFIED`).
   - **The reports are consistent in themselves** (review round, G185).
     - **Integrity report (`INTEGRITY_REPORT_INCONSISTENT`).** `integrity::check_report` re-derives its counts and verdict from its evaluations and requires it to cite the pinned envelope and evaluate every pinned invariant. It now also requires, for a HARD invariant, that a FAIL carry HARD_VIOLATION (or UNSELECTED_REVISION), that an UNKNOWN carry REQUIRED_UNKNOWN, and that the status never be CONFLICT or NOT_AFFECTED, which `evaluate` never writes; a CONFLICT does not silently pick a winner. Every evaluation must also be of a pinned invariant. These are the rules `integrity::evaluate` writes by.
     - **Verification report (`VERIFICATION_REPORT_INCONSISTENT`).** The new `verification::check_report` re-derives its coverage, blockers and verdict from its outcomes under its policy, through the same `summarize` that `verification::evaluate` now uses. A required class is covered only by a *required* obligation of that class: `summarize` itself changed, so `evaluate` writes the same rule. It requires its failures to name exactly the FAILED outcomes, every SATISFIED outcome to cite evidence, and no cited evidence id to be empty or carry a control character.
   - **The seal gate applies the same checks.** `seal::gate` takes the pinned envelope (`GateInputs::envelope`) and refuses `INTEGRITY_REPORT_INCONSISTENT` and `VERIFICATION_REPORT_INCONSISTENT` itself. `seal gate` (`runtime::seal::gate_container`) reads the envelope from `--root`'s `PINNED_ENVELOPE_PATH`. `seal gate` and the precondition therefore decide alike: a report the gate would refuse cannot be sealed by it, and a record forged to name one is refused by the precondition. This closes RES-G179-GATE-TRUSTS-INTEGRITY-VERDICT.

     The report contents themselves cannot be recomputed from the container: the integrity evaluations and the verification outcomes, and the evidence behind them. `integrity::evaluate` needs the candidate's ADL constraint results and its currently declared envelope. `verification::evaluate` needs the evidence runs. The container carries neither: it has facts, obligations, the declared nodes and edges, the certificate and typed records, but no ADL constraints, constraint results or verification evidence. These contents are taken as authored and listed in `NOT_VERIFIED`, step 5 (residual RES-G185-REPORT-CONTENTS-AS-AUTHORED).
   - **The gate decides the record.** This is the last check, and it runs only when every other check passed.
     - The gate's authority check runs first (`design::authority_violations` against the declared registry), in the gate's own words. A design without an authority event is refused `SEAL_GATE_NOT_ELIGIBLE: DESIGN_WITHOUT_AUTHORITY`, and one no declared principal selected is refused `SEAL_GATE_NOT_ELIGIBLE: DESIGN_AUTHORITY_REFUSED`, both before the container is re-encoded.
     - The precondition then recomputes the root of the container as it was before the seal. Sealing changes the root manifest, so the same content with the seal removed is written and read again, and that root is the root the design selected.
     - It runs `seal::gate` over that container, its certificate, the declared policy, both reports, the design and the declared registry.
     - It requires the decision to be ELIGIBLE. Otherwise it refuses `SEAL_GATE_NOT_ELIGIBLE`, once for each of the gate's typed reasons.
     - It requires the decided record to equal the record the container carries. Otherwise it refuses `SEAL_RECORD_NOT_DECIDED`, naming both seal ids.

   A self-certified record is therefore refused. To be admitted, it must be exactly what the gate decides over the repository's declared policy, envelope and principals, and over reports consistent with themselves and the pin. The design must carry a declared principal's signature, and the admission rests on that signature and on nothing Atlas holds. The gate is the same code as `seal gate`, so the seal decision exists in one place. The design's `parent_root` is compared by the gate itself, with the recomputed pre-seal root, which closes RES-G179-DESIGN-PARENT-ROOT-UNCOMPARED.

   **What an admission names.** `AdmittedParent` names who admitted it and under which declaration:
   - the declared `policy_id`;
   - the selecting `principal` and the `principal_key` (lowercase hex) its event is signed under;
   - the `registry_digest` (BLAKE3 of the declared registry).

   `--root` stays an operator choice, and these fields record it. `PreconditionInputs` is `{parent, design, verification, integrity, declaration}`, and a report that does not decode is `UNREADABLE` with its name. `admit` takes the decoded `Reports` and the `Declaration`. `NOT_VERIFIED` no longer lists step 7. For step 5, it states exactly what remains as authored.
2. **The materializer, first slice (M11): one class, staged.** `core::atlasx::materialize` takes the same `PreconditionInputs` and works as follows.
   - **M0/M1.** The precondition decides on the same bytes. The decoded parent and design are used only when they are ADMITTED. Otherwise the result is `PARENT_NOT_ADMITTED`, one defect for each precondition reason (`NOT_SEALED: ...`), and no object exists.
   - **M2, bounded to the design's roots.** Every root must resolve in the parent as a record of the dimension it names (`ROOT_UNRESOLVED`). This is the reachability part of contract step 8, for the roots.
     - Each FUNCTION_IDENTITY root selects one function. Its FUNCTION_SIGNATURE record is the one whose function identity key is the same.
     - None is `SIGNATURE_ABSENT`, and two different ones are `SIGNATURE_CONFLICT`. No signature is invented, and no winner is chosen.
     - A declaration kind that has no FUNCTIONS dispatch code is `DISPATCH_UNSUPPORTED`.
     - No function selected is `NOTHING_TO_MATERIALIZE`.
     - Roots of other dimensions are listed, sorted, in `not_materialized` as `DIMENSION:record`, never dropped silently.
     - A root the design repeats selects once. Two distinct functions that the provisional GLOBAL_ID cannot tell apart are `GLOBAL_ID_COLLISION`, and neither is chosen.
     - The signature records are indexed by function identity key once, not scanned per root.
   - **The record.** A FUNCTION_SIGNATURE record carries:
     - the function's name, owner type, dispatch, visibility, documentation, parameters (name and type spelling), result and body fingerprint, as the census recorded them;
     - its GLOBAL_ID, the construction IR's provisional form `fn:<path>::<owner>::<name>` (`fn:<path>::<name>` without an owner), the form ADR 0095's fixture uses;
     - its lineage, the census records it was lifted from: the FUNCTION_IDENTITY and FUNCTION_SIGNATURE record ids.

     The materialized `is_local` record equals, field for field, the one the G181 lifter produced for SR1-3 from an earlier census of the same source. Only the lineage differs, because the record ids are that census's.
   - **M7/M8.** The records go through `codec::encode_functions` into one canonical object. Its staging path is `functions/<address>.atlasx`, the contract's directory and content-address rule.
   - **M11, for this object.** The bytes are read back through `codec::decode`. `check_lineage` then requires every lineage entry to be a census record of the parent (`LINEAGE_OUTSIDE_PARENT`), and every record to include a FUNCTION_IDENTITY root the design selected (`LINEAGE_NOT_SELECTED`). A codec refusal, on write or on read back, is `ENCODING_REFUSED` with the codec's defect.
   - **The verdict.** STAGED or REFUSED (`MaterializationVerdict`), with typed, sorted `MaterializationDefect`s. The embedded precondition verdict binds the parent root, the seal, the envelope and the design. `functions` maps each GLOBAL_ID to its lineage, so the id mapping is explicit. Both enums are in the vocabulary map.
3. **Publication stops at staging.** The contract's transaction writes the objects to staging, verifies each one, then builds the manifest and root identity, and only then publishes. Here, `runtime::atlasx::materialize`:
   - refuses a staging directory that already holds anything, so no stale object mixes in, and creates the staging directory (its parent must exist) only when it is absent;
   - creates each object's directory with `create_dir`, never following one that already exists (a symbolic link included), and the object with `create_new`, then syncs it;
   - reads each object back and requires it to decode under its address name, so the file is the canonical object whose digest is that name;
   - removes every file and directory it created if any step fails, the staging directory included when it created it, and nothing it did not create;
   - writes nothing for a refused materialization.

   Steps 4 to 8 (manifest, AtlasX root identity, publish, advertise) are not done. ROOT_MANIFEST has no schema (RES-G183-SINGLE-CLASS). Every result lists what is not done (`MATERIALIZE_NOT_DONE`):
   - publication;
   - every other class;
   - selection closure beyond the roots;
   - bodies;
   - a stable GLOBAL_ID;
   - the records' epistemic status;
   - stages M3 to M6;
   - M11's deterministic-reproduction check;
   - M13 validation.

   A staged object is not an AtlasX root.
4. **CLI.** `atlasx precondition` and the new `atlasx materialize` take the same parent arguments: `--atlas`, `--design`, `--verification` and `--integrity`, and `--root`, whose declared seal policy, pinned envelope and principals are the `Declaration`. `--policy` and `--envelope` are refused ("does not take --policy: the repository at --root declares it").

   `atlasx materialize ... --out <staging>` prints the typed result and exits `ATLASX_MATERIALIZATION_REFUSED: <n> defects` when refused. A verification file in the `verification self` form (`{"evidence", "report"}`) is read by its report, as `seal gate` reads it.

## Evidence

- **Fixture.** The G161 fixture, shared by the precondition and materializer tests (`core/src/atlasx/fixture.rs`), now carries real census records. `core/src/atlasx/materialize/census_records.json` holds the FUNCTION_IDENTITY and FUNCTION_SIGNATURE records of `MaterializationMode::is_local` and the SYMBOL record of `MaterializationMode`, with their evidence. They are copied verbatim from the G184 self census container (root `8390d190…`, revision `c5fc38bd…`). The design selects `is_local`'s identity and the type's definition. A fixture principal signs it with a test key; Atlas signs nothing. The container is sealed by the real gate.
- **Falsification.**
  - **The G179 reproduction.** An unsealed census container gets a record with arbitrary policy and report ids, the manifest's certificate, census and revision copied, every blocker covered, and `seal_id = seal_identity(record)`, naming the pinned envelope and the SELECTED design. The M9 writer and reader accept it. Every G179 check passes, but it is refused with exactly `SEAL_RECORD_NOT_DECIDED`. With the gate re-run removed (mutant M01), it is ADMITTED. The same forgery over a design signed by an undeclared key is refused `SEAL_GATE_NOT_ELIGIBLE: DESIGN_AUTHORITY_REFUSED`.
  - **The legitimate G161 path is still ADMITTED**, with the same bound identities, and so is the fixture sealed by the gate for a design with other roots.
  - **Each gate input changed alone refuses.**
    - An admissible policy or a consistent report that the record does not name is `SEAL_RECORD_NOT_DECIDED`.
    - Each of the following is `SEAL_GATE_NOT_ELIGIBLE` with the gate's reason: a policy edited under its identity, a consistently BLOCKED or foreign verification report, a consistently REJECTED integrity report, and a registry that does not declare the selecting principal. The authority refusal comes alone, before the re-encode.
    - A verdict edited against its own report is refused `*_REPORT_INCONSISTENT` and never reaches the gate.
    - A design selected over the sealed root is `SEAL_GATE_NOT_ELIGIBLE: DESIGN_CANDIDATE_MISMATCH`.
  - **Review P1: reports a principal authors themselves.** Each dirty report below was sealed by the real gate, which reads the verdicts, and is refused by the precondition with its typed reason:
    - an integrity report with a HARD FAIL counted as none and verdict ELIGIBLE;
    - the same failure with its HARD_VIOLATION dropped;
    - an undecided HARD invariant without REQUIRED_UNKNOWN;
    - an ADMISSIBLE verification report naming `UNIT: 12 required unit obligations fail` among its blockers;
    - a failure naming an obligation that did not fail;
    - a SATISFIED outcome without evidence.

    A seal over an envelope its author wrote (consistent, ELIGIBLE, sealed by the gate) is admitted only under that envelope as the pin. Under the repository's pin it is refused `ENVELOPE_MISMATCH` and `INTEGRITY_REPORT_INCONSISTENT`. The reviewer's probe files, replayed through the new CLI, are refused with the same reasons.
  - **Re-review: the gaps closed.**
    - A HARD invariant decided CONFLICT or NOT_AFFECTED with verdict ELIGIBLE (the reviewer's `gap-int-conflict.json` and `gap-int-notaffected.json`) is refused by the gate and by the precondition.
    - A report whose required classes are covered only by FAILED obligations that are not required, ADMISSIBLE with no blockers (`gap-ver-allfailed.json`), is refused the same way.
    - `seal gate` through the CLI refuses the P1 integrity report `SEAL_NOT_ELIGIBLE` with `INTEGRITY_REPORT_INCONSISTENT`, so the two deciders agree.
    - Staging creates and removes the staging directory only when absent, and does not stage through an object directory that already exists as a symbolic link.
    - An event-less design is refused before the re-encode.
  - **Review P2: no override.** The policy, envelope and registry come only from `--root`. `--policy permissive.json` and `--envelope` are refused by the CLI. An admission names its policy id, principal, key and registry digest, and another registry gives another digest.
  - **The admitted fixture stages one FUNCTIONS object** at `functions/<address>.atlasx`. It decodes under that name to one record, `fn:core/src/donor/mod.rs::MaterializationMode::is_local`, whose lineage `[FUNCTION_IDENTITY:f9cc9be4f375986b, FUNCTION_SIGNATURE:43bada38908117fd]` resolves among the parent's records. `SYMBOL:02255955f298ded0` is listed as not materialized. The result is deterministic.
  - **Lineage that does not map back is refused.** An entry re-pointed outside the parent is `LINEAGE_OUTSIDE_PARENT`, and so is a parent without the records. A design that did not select the function, or lineage without the identity, is `LINEAGE_NOT_SELECTED`. Staging applies both checks to forged records.
  - **A parent the precondition refuses is refused before any object exists:** unsealed, without a design, self-certified, or not a container.
  - **Other refusals:**
    - unresolved and mis-dimensioned roots;
    - no function selected;
    - a missing signature and two conflicting signatures;
    - a trait-implementation method;
    - two distinct functions with one GLOBAL_ID (`GLOBAL_ID_COLLISION`);
    - a malformed body fingerprint (`ENCODING_REFUSED: MALFORMED_VALUE`).

    A repeated root selects once. A staged object that does not read back is refused, and staging leaves nothing behind.
  - **Through the CLI, end to end.** `seal gate`, then `atlas seal`, then `atlasx precondition` ADMITTED, then `atlasx materialize` STAGED. The staged file decodes with `atlasx codec --decode` under its address, with lineage into the container. A second run into the same staging directory is refused. A self-certified container is refused by the precondition (`SEAL_RECORD_NOT_DECIDED`). It and the unsealed container are refused by `atlasx materialize` (`ATLASX_MATERIALIZATION_REFUSED: 1 defects`), and no staging directory is created.
  - **The self scope stays refused.** `atlasx materialize` over the G184 self census container, under the repository's own declarations and with the G184 self-scope verification and integrity reports, is refused with `NOT_SEALED` and `DESIGN_ABSENT` and stages nothing. Both reports are consistent. Atlas itself is not materializable.
- **Mutation testing** (`evidence/verification/G185-atlasx-materialize.json`). Each mutant was applied alone, and each source file was restored and checked by sha256.
  - **First pass:** 29 mutants, 28 killed. The survivor was equivalent: a staged-file byte comparison, redundant with decoding under the address name, which was removed.
  - **Final code, after both review rounds:** 60 mutants, all killed.
    - The 28 first-pass mutants were re-run; one no longer applies because its text moved, and R21 mutates the same rule.
    - 23 cover the first review's fixes: report consistency, the pinned envelope, the admission's names, the authority pre-check, root deduplication, GLOBAL_ID collision, the signature index, staging cleanup and create-new, and the refused overrides. One of them, a coverage comparison removed, first survived until a coverage-only test was added.
    - 10 cover the re-review's fixes: HARD CONFLICT and NOT_AFFECTED, required coverage by a required obligation, the gate's two report checks, the event-less pre-check, object directories created and never followed, the staging directory's removal, and malformed evidence ids.

## Consequences

- **RES-G179-SEAL-SELF-CERTIFYING is closed.** This holds for every consumer that goes through the precondition, and the materializer does.
  - No record is admitted unless the gate decides it over the repository's declarations and over reports consistent with themselves and the pin, with a design signed by a declared principal.
  - What that principal can still author is the contents of the reports: outcomes and evaluations whose evidence the container does not carry. That remainder is RES-G185-REPORT-CONTENTS-AS-AUTHORED.
  - A SEALED container on its own still proves only that its record binds it: `atlas::write` and `atlas::read` are unchanged.
  - RES-G179-DESIGN-PARENT-ROOT-UNCOMPARED is closed with it.
- **M11 exists for one class, into staging.** An admitted parent's selected functions become a canonical FUNCTIONS object whose records map back to the parent's census records. There is no manifest, AtlasX root identity, publication, other class, closure beyond the design's roots, or body. M13 stays MISSING, and so does MIN_ATLASX. A staged object is not an AtlasX root, and nothing may report it as one.
- **The seal gate itself (M8, `seal gate`) no longer trusts a verdict its report contradicts.** It applies both consistency checks against the pinned envelope, the same checks the precondition applies, so RES-G179-GATE-TRUSTS-INTEGRITY-VERDICT is closed. Every historical report still passes the tightened checks: the 47 integrity reports under `evidence/census/*/integrity-report.json` (against the current pin) and the 58 self-scope verification reports under `evidence/verification/`. What stays authored is the reports' contents, not their verdicts (RES-G185-REPORT-CONTENTS-AS-AUTHORED).
- **The GLOBAL_ID stays provisional.** The materializer carries the construction IR's form, the one ADR 0095's fixture already uses, so no new object schema version is needed. A stable AtlasX identity form is still undefined, and a collision under it is a typed refusal.
- **A parent from an older container writer re-encodes under the current schema ids.** The pre-seal root then differs from the one the design selected, and the gate refuses `DESIGN_CANDIDATE_MISMATCH` (residual RES-G185-PRE-SEAL-ROOT-REENCODE).
