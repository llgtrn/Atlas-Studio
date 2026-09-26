---
id: atlas.decision.0072.resource-dimension
type: decision
status: accepted
canonical: true
---
# ADR 0072 — The RESOURCE dimension: acquisition and release (G157)

## Context

DEBT-RESOURCE has been open since the G118 audit, at M0_ABSENT. `SemanticDimension` had twelve dimensions and no RESOURCE:
- OWNERSHIP records moves and borrows;
- PERSISTENCE records durable reads, writes and syncs;
- CONCURRENCY records locks and spawns as operations.

None of them says when a file, a socket, a lock or a thread is acquired, or when it is given back. The construction milestone M14 (lowerable body semantics) and the physical domains both need that. FULL_OSS_REPLAY R3 (tree-sitter) added a concrete case to the debt's evidence: `ts_parser_new` and `ts_parser_delete` acquire and release across the FFI boundary, and Atlas had nowhere to record either.

## Decision

1. **RESOURCE is the thirteenth semantic dimension.**
   - `core::semantic::resource` defines `ResourceIdentity`, which records a function, an operation (ACQUIRE or RELEASE), a kind (FILE, SOCKET, LOCK_GUARD, THREAD) and a span.
   - A release also names:
     - the acquisition it gives back (`acquired_at`);
     - its holder;
     - how it happens (SCOPE_END, EXPLICIT_DROP, JOIN).
   - `is_well_formed` refuses an acquisition with release fields.
   - It also refuses a release that the kind's semantics forbid. A THREAD is never released by dropping its `JoinHandle`, since that detaches the thread. A JOIN is only for a thread.
2. **Acquisition is DERIVED from name resolution.**
   - The path resolver resolves a call to a std path.
   - The declared std-path resource table (`std_path_resource`) names what that path acquires:
     - `File::open`, `File::create` and `File::create_new`;
     - TCP, UDP and Unix sockets;
     - `Mutex::lock`, `RwLock::read` and `RwLock::write` on a receiver whose std type is known;
     - `thread::spawn`.
   - The record is anchored at the call. This is the G77/G117/G125 pattern: name resolution fixes the callee, and the std contract fixes the meaning.
3. **Release is claimed only for a holder that the resolver can follow.** The holder is a local bound once by a `let`, and its initializer is the acquiring call, possibly through `?`, `.unwrap()` or `.expect(..)`. In the block the holder lives in:
   - Some uses leave the holder in place:
     - a borrow;
     - a field;
     - an index;
     - a dereference;
     - a method receiver, except a method that takes the holder by value (`take`, `bytes`, `chain`, `into`, `try_into`, `into_*`);
     - a formatting or assertion macro.
   - Some uses release it, as a statement of that block:
     - `drop(holder)` resolved to `std::mem::drop` is EXPLICIT_DROP, DERIVED;
     - `holder.join()` resolved to `JoinHandle::join` is JOIN, DERIVED.
   - Any other use moves the holder, and no release is claimed for it:
     - a value passed, returned, stored or assigned, or consumed by a by-value method;
     - a `move` closure naming the holder;
     - the holder named inside any other macro;
     - a release inside a branch, block, closure or match arm.
   - A holder that is never moved is released at the end of its block (SCOPE_END), when dropping releases the resource. This record is INFERRED, because the move check is syntactic and not borrowck. The recorded point is the closing brace. An early return or an unwind releases the holder sooner, on the same drop.
4. **Unknown release is not "no release".**
   - An acquisition with no release record has an unknown release point. Examples are a temporary guard or a holder that was moved.
   - The RESOURCE obligation of every Rust artifact stays UNKNOWN, and its diagnostic names what is outside the profile:
     - temporaries;
     - moved holders;
     - fields and statics;
     - non-std resources;
     - FFI pairs such as `ts_parser_new` and `ts_parser_delete`;
     - macro arguments.
5. **The dimension flows everywhere a dimension flows.**
   - The `.atlas` container gains a `resource` subject kind and a `Resource` record kind, recorded in a new schema-history generation (G157).
   - The composed world model gains per-function resource sites.
   - The engineering graph, the seal's dimension set, design comparison and the path-resolution engine's evaluated dimensions all gain RESOURCE.

## Consequences

- DEBT-RESOURCE moves from M0_ABSENT to a partial state. Acquisition is evaluated by one engine. Release is claimed for let-bound holders.
- Reconciliation with OWNERSHIP's move records, FFI acquisition and release pairs, and non-std resources remain open. Those, and a borrowck or MIR oracle for the move check, are the next attacks on the debt.
- The census of Atlas Studio records the resources Atlas itself acquires and gives back, and a fixture exercises every release rule and every refusal.
