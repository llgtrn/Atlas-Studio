---
id: atlas.reports.donor-pin-verification-2026-09-20
type: report
status: active
canonical: true
---
# Donor Pin Verification

## Existing 24 Donors

All 24 pre-existing donors under `.atlas/temporary/donors/` were checked against `.atlas/provenance/donors/donor-clone-index.json` on `2026-09-20`.

Result: every donor checkout HEAD matched the recorded `commit_sha`.

Verified IDs:

- `ast-grep`
- `buck2`
- `c2rust`
- `crubit`
- `cytoscape-js`
- `datafrog`
- `differential-dataflow`
- `elkjs`
- `glean`
- `joern`
- `kani`
- `kythe`
- `miri`
- `openrewrite`
- `py2many`
- `rust-analyzer`
- `salsa`
- `scip`
- `semgrep`
- `souffle`
- `sourcetrail`
- `tree-sitter`
- `verus`
- `xyflow`

## Newly Cloned Approved Donors

| Donor | Clone path | Commit SHA | Tree hash | Status |
| --- | --- | --- | --- | --- |
| `opendesign` | `.atlas/temporary/donors/opendesign` | `b4e69ac61b50576298f9f564603e5a4beb27417f` | `9e93376bfd915ddb2df03ed6ac1d043e6141c3ae` | `CLONED` |
| `containers-image` | `.atlas/temporary/donors/security/containers-image` | `df7e80d2d19872b61f352a8a182ec934dc0c2346` | `a64f45afb94ccd84c954c73819f3d5f0d4162429` | `CLONED` |
| `podman` | `.atlas/temporary/donors/security/podman` | `0d12a23e7bc94460f7986c01c79636b63ab6cd36` | `cf105b1467f82ba5f91de7a8ae93a132472e8d66` | `CLONED` |
| `selinux` | `.atlas/temporary/donors/security/selinux` | `3361233fa3cb0c27f68f682f195accefb69d06c0` | `4956356e63224b7d9bdfa373312ff06ed6f14094` | `CLONED` |
| `openscap` | `.atlas/temporary/donors/security/openscap` | `6942b59fc861ad77ebcea93e70f32832d88e20f9` | `5eb0b3b74b55ee913824d90f630ca70ef672869a` | `CLONED` |
| `keycloak` | `.atlas/temporary/donors/security/keycloak` | `dd4ae31d1b67c91a7f85f7c60df5c9718b111f0a` | `6e60572de3133ad0507ceab54f6a0361a1ffd0ec` | `CLONED` |
| `keylime` | `.atlas/temporary/donors/security/keylime` | `3476366881d7931407154597d88577895f2f818e` | `13efa0c0d0f8eef83118111139fcf219ae2b141b` | `CLONED` |
| `clair` | `.atlas/temporary/donors/security/clair` | `c9ac9a0aa5651670a690854fc2c12adfc951243f` | `944ab8e839025e23f6f13ad86a3c4143b4174d54` | `CLONED` |

Security note: clone success, license capture and a pinned commit are provenance facts only. They do not make any donor trusted for ingestion or ATLAS compilation.
