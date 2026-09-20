---
id: atlas.standard.compiler-optimization
type: contract
status: active
canonical: true
---
# Compiler Optimization Standard

Optimization priority is semantic and whole-system first, machine-local second.

## Ordered optimization layers

1. graph/binding specialization;
2. capability/interface devirtualization;
3. state placement and partitioning;
4. memory ownership/lifetime/region selection;
5. data-layout and locality optimization;
6. concurrency/conflict scheduling;
7. partial evaluation and deployment-policy specialization;
8. cross-module/repository fusion where semantic boundaries allow;
9. HIR/MIR scalar and control-flow optimization;
10. vectorization/SIMD/GPU specialization;
11. instruction selection/register allocation/scheduling;
12. LTO/whole-program optimization;
13. link/post-link layout;
14. PGO and empirical auto-tuning.

## Performance objectives

A compiler invocation MUST use an explicit objective/profile rather than assuming one universal optimum. Possible objectives include throughput, p50/p95/p99 latency, deadline/jitter, memory, startup, binary size, energy, GPU utilization or a weighted/multi-objective policy.

## Reproducibility

Every optimization decision that depends on profile or target facts must record those inputs. Heuristic decisions should be queryable from compiler evidence.

## Verification

High-impact transformations require differential tests, invariant checks and/or benchmark evidence appropriate to the risk class. Faster output that violates semantic equivalence is invalid.
