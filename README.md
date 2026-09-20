# Atlas Studio

Atlas Studio is a product-neutral engineering-world compiler and system invention environment.

It ingests repositories, donor OSS, specifications, DeepWiki-style repository explanations, scientific papers, tests, benchmarks and other admitted evidence; performs adaptive multi-scope census; synthesizes a typed engineering world; and materializes verified native systems.

The canonical engineering pipeline is:

```text
source reality / donors / research / declared intent
                    ↓
             secure admission
                    ↓
         adaptive semantic census
                    ↓
          design + synthesis loop
                    ↓
        <system>.atlas
 dense canonical engineering artifact
                    ↓
          Atlas materializer/compiler
                    ↓
        <system>.atlasx/
 expanded executable repo representation
                    ↓
             compiler backend
                    ↓
       executable / library / UI bundle
                    ↓
          test / benchmark / recensus
                    ↺
```

Important naming distinction:

- `.atlas/` is the repository knowledge/control directory.
- `*.atlas` is Atlas Studio's dense binary engineering artifact.
- `*.atlasx/` is the deterministic expanded executable representation produced from a selected `*.atlas` design.

Atlas does not census OSS to translate it line-for-line. Donors are evidence. Atlas extracts mechanisms, algorithms, invariants, trade-offs and technology primitives, synthesizes a target-native design, and then materializes that design.

Every Atlas-generated repository MUST implement the same universal engineering graph contract: globally stable identities, nodes, edges, bindings, scopes, state/event semantics, temporal revisions, evidence/provenance, constraints/invariants, interfaces/capabilities, effects and materializations. This allows independently generated repositories to federate into one engineering world without becoming one mutable monorepo or competing truth systems.

## Current compatibility boundary

The current compatibility binary remains `atlas-systemizer` and the compatibility API remains `atlas.systemizer.cli.v1` while implementation migrates toward the Atlas compiler architecture.

## Implementation policy

- Backend/compiler/runtime: Rust.
- Frontend: TypeScript/TSX.
- C is permitted only at explicit low-level/foreign ABI boundaries unless a later compiler phase proves a native target requirement.
- `.atlas/` is the sole repository knowledge/control root.
- Donor OSS is evidence and technology reference, not permanent runtime substrate.
- Generated or inferred claims are not admitted as verified fact without evidence.
- Current donor absorption continues to materialize Rust backend and TypeScript frontend until later compiler phases replace that lowering path.
