---
id: atlas.contract.compiler-ir-schemas
type: contract
status: active
canonical: true
---
# Atlas Compiler IR Schemas v1

## Purpose

This contract defines the minimum canonical logical schemas for Atlas HIR, MIR, LIR and Machine IR v1.

It complements `COMPILER-IR-PIPELINE.md`:

- that contract defines stage responsibility and lowering boundaries;
- this contract defines the records/operation families each stage must expose.

Implementations may use optimized in-memory layouts, but their externally verifiable semantics MUST map losslessly to these logical schemas.

## Common IR identity rules

Every identity-bearing IR record carries:

- `ir_schema_version`;
- stable deterministic `id`;
- parent stage lineage reference when applicable;
- source AtlasX identity/lineage root;
- semantic-barrier references;
- target/profile references when stage-relevant.

Display names are never sufficient identity.

Stage-local IDs MUST be deterministic for the same pinned compiler inputs.

## Common barrier reference

Every stage may attach zero or more typed barriers:

~~~text
AUTHORITY
SAFETY
EFFECT
TRANSACTION
PERSISTENCE
RECOVERY
CONCURRENCY_ORDER
TEMPORAL_LIFECYCLE
EXTERNAL_INTERFACE
PROVIDER_ADMISSION
~~~

Barrier identity refers back to AtlasX/parent-stage semantics.

Unknown generic strings are not mature barrier representation.

# HIR v1

## HIR role

HIR is the highest compiler-owned semantic IR.

It preserves selected executable semantics while removing AtlasX packaging/presentation detail.

## HIR root

A HIR module set contains:

~~~text
HirProgram
├─ schema_version
├─ program_id
├─ atlasx_root_id
├─ target_constraint_refs[]
├─ modules[]
├─ types[]
├─ functions[]
├─ interfaces[]
├─ capabilities[]
├─ bindings[]
├─ external_boundaries[]
└─ barrier_refs[]
~~~

## HirModule

Required fields:

- id;
- parent module id or null;
- stable semantic/source lineage;
- member type ids;
- member function ids;
- interface/capability ids;
- exported identities;
- constraints/barriers.

## HirType

Required fields:

- id;
- source AtlasX type identity;
- kind;
- generic parameter definitions;
- field/variant/member structure;
- ownership/resource constraints;
- representation constraints already semantic in SelectedDesign;
- interface/ABI constraints already selected;
- lineage.

Core HIR type kinds:

~~~text
PRIMITIVE
STRUCT
ENUM
TUPLE
ARRAY
SLICE
REFERENCE
POINTER
FUNCTION
TRAIT_INTERFACE
OPAQUE
RESOURCE
GENERIC_PARAM
GENERIC_APPLY
EXTERNAL
~~~

## HirFunction

Required fields:

- id;
- source AtlasX function identity;
- declaration/dispatch kind;
- parameter ids/types;
- result type;
- generic parameters;
- body root;
- declared effects;
- resource/ownership requirements;
- concurrency requirements;
- persistence requirements;
- external boundary refs;
- barrier refs;
- lineage.

## HIR body node

HIR bodies use structured nodes.

Every node contains:

- id;
- node kind;
- result type/value id if any;
- child/value references;
- barrier refs;
- lineage.

Core HIR node kinds v1:

~~~text
CONST
LET
ASSIGN
CALL
DYNAMIC_CALL
IF
MATCH
LOOP
BREAK
CONTINUE
RETURN
FAIL
BLOCK
STATE_READ
STATE_WRITE
STATE_TRANSITION
EFFECT
RESOURCE_ACQUIRE
RESOURCE_RELEASE
RESOURCE_TRANSFER
BORROW_SHARED
BORROW_MUT
MOVE
COPY
SPAWN
AWAIT
CHANNEL_SEND
CHANNEL_RECV
LOCK
UNLOCK
ATOMIC
TX_BEGIN
TX_COMMIT
TX_ABORT
PERSIST
EXTERNAL_CALL
CAST
AGGREGATE
FIELD_GET
FIELD_SET
INDEX
INTRINSIC
~~~

A construct outside this set requires a versioned schema extension or explicit unsupported compiler diagnostic.

## HIR verification

HIR verifier MUST reject:

- unresolved required type/function references;
- broken binding targets;
- missing body for required executable function;
- illegal dynamic call without dynamic-binding policy;
- resource operation violating selected ownership requirement;
- missing required barrier;
- impossible selected state transition;
- untyped node result where schema requires type.

# MIR v1

## MIR role

MIR makes control flow, values, memory/resource operations and failure paths explicit.

## MirProgram

~~~text
MirProgram
├─ schema_version
├─ program_id
├─ parent_hir_program_id
├─ functions[]
├─ global_state[]
├─ externs[]
├─ layout_constraints[]
└─ barrier_refs[]
~~~

## MirFunction

Required fields:

- id;
- parent HIR function id;
- function type/signature;
- entry block id;
- ordered block ids;
- local/value type table;
- resource/ownership summary;
- effect summary;
- barrier refs;
- lineage.

## MirBlock

Required fields:

- id;
- deterministic block ordinal;
- block parameters;
- instruction ids in semantic execution order;
- exactly one terminator;
- predecessor ids optionally cached but derivable;
- barrier refs.

## MirValue

Required fields:

- id;
- type id;
- value class;
- defining instruction/block parameter;
- ownership/resource class when relevant.

Core value classes:

~~~text
SSA_VALUE
BLOCK_ARGUMENT
ADDRESS
REFERENCE
RESOURCE_HANDLE
STATE_HANDLE
FUNCTION_REF
CAPABILITY_REF
EXTERNAL_REF
~~~

## MirOperand

Core operand forms:

~~~text
VALUE(id)
CONST(id)
GLOBAL(id)
FUNCTION(id)
BLOCK(id)
TYPE(id)
CAPABILITY(id)
EXTERNAL(id)
~~~

## MirInstruction

Every instruction contains:

- id;
- opcode;
- result ids;
- operand list;
- type refs;
- barrier refs;
- effect/resource metadata when opcode requires;
- lineage to HIR node(s).

Core MIR opcodes v1:

~~~text
CONST
COPY
MOVE
BORROW_SHARED
BORROW_MUT
CAST
AGGREGATE
EXTRACT
INSERT
ALLOC
FREE
LOAD
STORE
ADDRESS_OF
CALL
DYNAMIC_CALL
INTRINSIC
READ_STATE
WRITE_STATE
STATE_TRANSITION
EFFECT
RESOURCE_ACQUIRE
RESOURCE_RELEASE
RESOURCE_TRANSFER
LOCK
UNLOCK
ATOMIC_LOAD
ATOMIC_STORE
ATOMIC_RMW
ATOMIC_CMPXCHG
FENCE
CHANNEL_SEND
CHANNEL_RECV
TASK_SPAWN
TASK_JOIN
AWAIT
TX_BEGIN
TX_COMMIT
TX_ABORT
PERSIST
FFI_CALL
EXTERNAL_CALL
MEMCPY
MEMMOVE
MEMSET
ASSERT
ASSUME
NOP
~~~

No implementation may encode a semantically meaningful operation as generic NOP/INTRINSIC merely to avoid defining the correct opcode, unless the intrinsic identity itself is a contracted semantic boundary.

## MirTerminator

Exactly one terminator per block:

~~~text
BRANCH
COND_BRANCH
SWITCH
RETURN
FAIL
PANIC
THROW
RESUME
SUSPEND
UNREACHABLE
~~~

Each terminator explicitly names successor blocks/returned values/failure payloads as applicable.

Fallthrough is not implicit.

## MIR ownership/resource rules

MIR distinguishes:

- value copy;
- semantic move;
- shared borrow/reference;
- mutable borrow/reference;
- raw/address operation;
- resource acquisition/release/transfer.

Unknown alias/ownership information is conservative.

The optimizer MUST NOT assume no-alias/free-reorder merely because analysis is absent.

## MIR state/effect rules

State identities and generic memory addresses are distinct.

`READ_STATE`/`WRITE_STATE`/state-transition operations preserve canonical state semantics even when later lowered to loads/stores/runtime calls.

Effects remain typed until their physical mechanism is chosen.

## MIR concurrency rules

Synchronization operations carry explicit ordering.

Atomic operations MUST include ordering enum:

~~~text
RELAXED
ACQUIRE
RELEASE
ACQ_REL
SEQ_CST
~~~

If target lowering cannot support required ordering directly, it must use a legal runtime/sequence or fail.

## MIR verification

Verifier MUST establish at least:

- entry block exists;
- block ids unique;
- every block has one terminator;
- successor ids resolve;
- value use is defined or is a block/function input;
- result types match opcode constraints;
- call signatures match;
- move/resource legality under represented semantics;
- state/effect/barrier references resolve;
- atomic order valid;
- transaction nesting/legal transitions valid where statically required;
- no required external boundary is hidden.

# LIR v1

## LIR role

LIR commits MIR semantics to explicit target-family layout/ABI/runtime mechanisms while remaining above exact machine instruction choice.

## LirProgram

Required fields:

- schema version;
- program id;
- parent MIR program id;
- target description id;
- ABI profile id;
- concrete type-layout table;
- functions;
- globals;
- extern symbols;
- runtime requirements;
- barrier refs.

## LirTypeLayout

Required fields where applicable:

- semantic type id;
- size;
- alignment;
- field offsets;
- discriminant/tag layout;
- address space;
- vector shape;
- ABI classification;
- niche/packing decision if used;
- lineage/proof reference.

Layout choices that violate SelectedDesign/ABI constraints are illegal.

## LirFunction

Required fields:

- id;
- parent MIR function id;
- concrete ABI signature;
- calling convention;
- ordered blocks;
- stack/local requirements abstractly known before Machine IR;
- unwind/failure strategy;
- barrier refs;
- lineage.

## LirInstruction

Core LIR opcode families v1:

~~~text
IADD ISUB IMUL IDIV IREM
FADD FSUB FMUL FDIV
AND OR XOR SHL SHR
CMP
SELECT
CAST
PTR_ADD
LOAD
STORE
MEMCPY
MEMMOVE
MEMSET
CALL
TAIL_CALL
INTRINSIC
ATOMIC_LOAD
ATOMIC_STORE
ATOMIC_RMW
ATOMIC_CMPXCHG
FENCE
VECTOR_OP
RUNTIME_CALL
FFI_CALL
EXTERNAL_CALL
STACK_ALLOC
HEAP_ALLOC
HEAP_FREE
TRAP
DEBUG_VALUE
~~~

Concrete widths/types are explicit operands/type refs.

## LIR terminators

~~~text
BR
COND_BR
SWITCH
RET
TRAP
UNWIND
UNREACHABLE
~~~

## LIR external symbol

An external symbol record contains:

- stable external-boundary lineage;
- symbol identity/name encoding;
- ABI profile;
- calling convention;
- parameter/return classification;
- ownership/resource transfer contract;
- required version/linkage;
- failure/unwind contract.

## LIR verification

Verifier MUST check:

- all concrete layouts satisfy target/ABI;
- calls match concrete ABI signatures;
- alignments/offsets are legal;
- atomic/order requirements are target-supported or runtime-lowered;
- external symbols have explicit ABI;
- no high-level semantic barrier is lost;
- every MIR operation is accounted by one or more LIR operations or proven eliminated.

# Machine IR v1

## Machine IR role

Machine IR is target-instruction-level representation used for instruction selection, register allocation, scheduling and object emission.

## MachineProgram

Required fields:

- schema version;
- program id;
- parent LIR program id;
- exact target description id;
- ABI profile id;
- CPU feature set;
- functions;
- globals/constant pools;
- symbol/relocation requirements;
- debug/unwind requirements.

## MachineFunction

Required fields:

- id;
- parent LIR function id;
- calling convention;
- ordered machine blocks;
- frame requirements;
- callee-saved/clobber requirements;
- unwind info requirements;
- stage state;
- lineage.

Machine stage state:

~~~text
PRE_RA
POST_RA
SCHEDULED
EMISSION_READY
~~~

## MachineBlock

Required fields:

- id;
- deterministic ordinal;
- parent LIR block refs;
- instruction ids;
- successor block ids;
- frequency/profile metadata when admitted;
- scheduling constraints.

## MachineInstruction

Required fields:

- id;
- target opcode id;
- operand list;
- explicit defs;
- explicit uses;
- implicit defs/uses;
- clobbers;
- instruction flags;
- barrier refs;
- source LIR lineage.

Target opcode id MUST resolve in the exact target description.

## MachineOperand

Core operand kinds:

~~~text
VIRTUAL_REGISTER
PHYSICAL_REGISTER
IMMEDIATE
MEMORY
STACK_SLOT
SYMBOL
BLOCK
CONSTANT_POOL
FRAME_INDEX
METADATA
~~~

## Register model

Target description defines:

- register classes;
- aliases/subregisters;
- reserved registers;
- caller/callee-save sets;
- operand constraints;
- allocation exclusions.

PRE_RA may contain virtual registers.

POST_RA executable operands requiring registers must resolve to legal physical registers or explicit spill/reload sequences.

## Memory operand

A memory operand explicitly identifies:

- address mode/base/index/scale/displacement;
- address space;
- access width;
- alignment;
- volatility/atomicity where applicable;
- alias class metadata if proven;
- lineage.

## Machine barriers

Instruction scheduling/reordering MUST obey surviving barriers and target ordering constraints.

A barrier may lower to:

- instruction sequence;
- fence;
- call boundary;
- scheduler constraint;
- "may not reorder across" metadata.

It may not silently disappear unless equivalence proof establishes it is redundant.

## Machine IR verification

Before object emission verifier MUST check:

- all opcodes exist for target;
- operand classes satisfy opcode constraints;
- all branches resolve;
- register allocation valid;
- no illegal reserved-register use;
- frame/stack alignment obey ABI;
- calls obey ABI;
- clobbers modeled;
- atomics/fences satisfy ordering;
- relocation/symbol forms supported;
- required unwind/debug data accounted;
- all surviving semantic barriers honored.

# Canonical stage hashing

Each IR stage may compute a deterministic stage root:

~~~text
IRRoot =
H(
  stage_schema_version
  parent_stage_root_or_atlasx_root
  target/profile inputs relevant to stage
  canonical ordered record content
)
~~~

The exact hash framing is implementation-versioned but MUST be deterministic and collision-resistant.

Cache identity may include compiler version/pass configuration in addition to semantic root.

# Serialization and evidence

IRs are compiler-internal canonical semantics, not durable product truth like ATLAS.

They MAY use a compiler-private efficient binary serialization for:

- incremental cache;
- reproducibility;
- differential verification;
- debugging/evidence.

Any serialized form used for cross-version persistence MUST carry:

- stage/schema version;
- compiler version;
- parent root;
- target/profile identities;
- integrity hash.

Unknown incompatible versions are rejected.

# Optimization legality

A pass may:

- replace record sequences;
- merge/split blocks;
- clone/specialize functions;
- eliminate proven redundant operations;
- change physical layout at legal stages.

It MUST provide deterministic lineage for transformed identity-bearing entities.

It MUST NOT:

- remove required barrier;
- change externally observable selected semantics;
- invent a provider/binding;
- exploit unknown ownership/alias facts as if proven;
- weaken transaction/durability semantics;
- change ABI without explicit target/design revision.

# Blueprint evolution

These v1 schemas are current canonical compiler blueprints, not immutable forever.

Census may discover superior:

- operation models;
- region/value representation;
- effect/barrier encoding;
- SSA form;
- ownership representation;
- LIR abstraction level;
- Machine IR structure.

A materially better schema MAY replace/supersede v1 through `BLUEPRINT-EVOLUTION.md`.

Such a revision requires:

- current-vs-candidate comparison;
- migration/compatibility analysis;
- updated lowering/verifier contracts;
- differential verification;
- targeted recensus/recompilation of affected reference workloads.

Implementation code may not silently reinterpret a v1 opcode/field.

# Final invariant

For every supported selected semantic construct there is one explicit path:

~~~text
AtlasX object
→ HIR record/node
→ MIR operations/CFG
→ LIR target/ABI operations
→ Machine instructions
→ object bytes
~~~

At no stage may required meaning disappear into an undefined "backend detail".
