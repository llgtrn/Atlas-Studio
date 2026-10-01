---
id: atlas.decision.0102.confined-sandbox-with-per-run-toolchain-identity
type: decision
status: accepted
canonical: true
---
# ADR 0102 — The confined sandbox class, probed per run, with per-run toolchain identity (G189, NA-SANDBOX-FS-CONFINEMENT, M22)

## Context

ADR 0047 gave Atlas a restricted-subprocess sandbox. That class clears the environment, stages digest-verified inputs, closes stdin, applies a timeout and can deny the network. It never claimed `FILESYSTEM_CONFINED`: an absolute or `..` path still reached the host. `DEBT-SANDBOXED_EXECUTION` stayed partial, because its success condition (a sandboxed build cannot read outside its declared inputs) did not hold. ADR 0100 then made M22 its own construction node: a confined sandbox run with per-run toolchain identity. M16 (the confined delegated build) requires M22.

The rules in force:

- Atlas contains no `unsafe` code and adds no dependency without a decision.
- `EXTERNAL-PROVIDER-TRUST.md` ("Sandboxed execution output") admits sandbox output as OBSERVED evidence only when the executed artifact identity is pinned and the sandbox and environment identity is recorded. It also says execution capability is discovered, never assumed.

Host facts, measured in this container (Linux 6.18, root):

- util-linux 2.39.3 `unshare` exists, and `unshare --user --map-root-user --mount` works.
- There is no bubblewrap.
- `/sys/kernel/security/lsm` is absent, so whether Landlock is usable is unknown. Landlock would need raw syscalls, which means `unsafe` code or a dependency.

## Decision

### 1. A confined class beside the restricted one

The confined class shares the restricted class's code for staging, timeouts and records. A request selects it declaratively with `SandboxRequest::confinement`, which carries the toolchain paths and the `proc` flag.

- A request with `confinement` runs under backend `atlas.sandbox.confined-subprocess.v1`.
- A request without it runs exactly as before, under `atlas.sandbox.restricted-subprocess.v1`.

Why a second class and not a replacement: the restricted class needs no namespaces and stays usable where they are refused. Because the class is part of the request, it is also part of the request digest, so a consumer that needs confinement asks for it and can check for it.

### 2. Mechanism

External util-linux binaries make every namespace, mount and chroot. No string is interpolated into a shell: a constant `sh` script (`CONFINE_SCRIPT`) receives every value as a positional parameter.

1. **Outer namespaces.** `unshare --user --map-root-user --mount --pid --fork --kill-child --propagation private [--net]` runs the script with no environment, from `/`. `private` is also `unshare`'s default for `--mount`; the flag is explicit, and the run's own mountinfo is tested to carry no peer group. Inside the namespace, the script:
   - mounts a private tmpfs on the per-run `root` directory;
   - binds the stage at `/work` (`nosuid,nodev`, the only writable path) and `/dev/null`;
   - binds each declared toolchain path at the same path (`ro,nosuid,nodev`);
   - mounts `/proc` only when asked (see below);
   - remounts the tmpfs read-only;
   - exports exactly the declared environment;
   - runs `exec "$@"`.
2. **The final command line.** Atlas builds it as argv: `unshare --user --map-user=1000 --map-group=1000 --root=<root> --wd=/work -- <program as found> <args>`. The program therefore runs chrooted, as uid 1000 of a nested user namespace, with no capability: exec as a non-root uid clears them all. It holds no capability over the mount namespace, so it cannot remount a bind writable. It holds no `CAP_SYS_CHROOT`, so it cannot use the double-chroot escape. The kernel refuses `unshare(CLONE_NEWUSER)` inside a chroot, so it cannot regain capabilities that way.
3. **Why chroot and not `pivot_root`.** After a pivot, any further command (the cap-dropping step, the unmount of the old root) must exist inside the new root. Atlas cannot run its own code there without `unsafe` code or a dependency. Chroot with no capability gives the same read boundary through a different mechanism. Its residue (the host mount tree still exists in the mount namespace, below the chroot) is listed under Consequences.
4. **The program is pid 1 of its own pid namespace.** A descendant that leaves the process group still dies at the timeout. The restricted class records that it cannot do this.

### 3. Root layout

- `/`: tmpfs, read-only, `nosuid,nodev`, mode 0755.
- `/work`: the stage (digest-verified copies of the declared inputs), read-write.
- `/dev/null`.
- Each declared toolchain path, at its host path, read-only, `nosuid,nodev`. A declared path that is itself a link (`/lib -> usr/lib`) is created in the root as the same link, not bound with its target's content, so paths beneath it, `..` included, resolve as on the host. Its target must lie under another declared path (`/usr/lib`), or the declaration is refused. Every other declared path must be canonical, with no link among its parents: a declaration passing through a link is refused, and the resolved path must be declared instead. `ln` joins the helpers.
- `/proc`: only when `confinement.proc` is set. It is a procfs with `subset=pid` of the run's own pid namespace, `nosuid,nodev,noexec`, so it shows the run's own processes and no `/proc/sys`.
  - It exists because `rustc` needs it: glibc's loader resolves `rustc`'s `$ORIGIN` runpath through `/proc/self/exe`.
  - The host's `/proc` is never shown. Its `/proc/<pid>/root` would reach host roots, because the program keeps the caller's kernel uid.

Nothing else exists: no `/etc`, `/home`, `/tmp`, repository or host stage path.

**What a declaration may be.** A toolchain path must be:

- absolute, normalized, existing, and a directory or a regular file;
- not inside another declared toolchain path;
- not covering `/work`, `/dev` or `/proc`;
- not overlapping the staging directory in either direction;
- not overlapping a directory of sockets and live endpoints: `/run`, `/var/run`, `/tmp`, `/var/tmp` or `/dev/shm`, whether as declared or once resolved, and whether the path is one of them, lies inside one, or contains one.

A read-only bind does not stop `connect()` on a unix socket. A declared directory elsewhere that holds a socket therefore still exposes it (RES-G189-TOOLCHAIN-SOCKETS).

The program must lie under a declared toolchain path twice over: the absolute path it was found at, and the file that path resolves to (see §5). Inside the root, the found path then resolves through the same binds to the same file, and anything undeclared on the way is absent (`PROGRAM_OUTSIDE_TOOLCHAIN`). The staging directory is created when missing (as before G189) and canonicalized before anything else, and must be a normalized absolute path, so the helpers never receive a relative path or one they would read as an option. A staging directory that cannot be created or resolved is a host error (`io::Error`), not a refusal of the request.

**The environment.** The nested `unshare` receives the declared environment so that the program does. The class therefore refuses environment names that are not plain identifiers, and names the pre-chroot loader or C library would act on: `LD_*`, `GCONV_PATH`, `GLIBC_TUNABLES`, `LOCPATH` and `NLSPATH`. The refusal is `ENVIRONMENT_REFUSED`.

### 4. The probe rule

`FILESYSTEM_CONFINED` is `ENFORCED` only when the mechanism was applied and a probe run in the same confinement passed. The probe uses the same helper, binds, namespaces, flags and stage as the run. In it, the kernel is asked to execute paths:

- a non-executable control file staged at `/work/.atlas-probe-control` must answer `EACCES` (exit 126);
- each of these must answer `ENOENT` (exit 127):
  - a per-run host sentinel beside the stage;
  - the control's own host path;
  - with a procfs, the sentinel through `/proc/1/root`.

Each answer must carry `unshare`'s exact `failed to execute <path>: <error>` line.

The probe exports the declared environment exactly as the run does. The two helper invocations differ only in the path executed at the end. `atlas-systemizer sandbox probe`, which has no request, probes with an empty environment.

When this host cannot confine, nothing runs: no `unshare`, a refused namespace, a failed helper, or any other answer from the probe. The run is `REFUSED` with `refusal_kind: CONFINEMENT_UNAVAILABLE`, and its record carries `FILESYSTEM_CONFINED: UNENFORCED{reason}` with the exact reason (`unshare` not found at a path, a mount error, the probed path that answered wrongly).

This is fail-closed, unlike the restricted class's network denial, which runs and records `UNENFORCED`. A request that asks for confinement is never run unconfined in its place. Availability alone is never a claim: `atlas-systemizer sandbox probe` reports the confined class by running the same probe.

### 5. Per-run toolchain identity

Every run that runs, in either class, records `toolchain: ToolchainIdentity`:

- for the program: the path found on the declared `PATH` (or given as a normalized absolute path), that path with every link resolved, and the BLAKE3 digest of the resolved file (the digest function of declared inputs);
- the same for each `toolchain_binaries` entry (absolute; in the confined class, under a declared toolchain path);
- when the program resolves to a toolchain's own `rustc` or `cargo` (verified by the sysroot rule below, whatever name it was found as), the standard output of `<program> -vV`. It runs the way the run executes: same class, same path, same declared environment. For `cargo`, `-vV` identifies cargo itself, not the compiler it invokes (RES-G189-TOOLCHAIN-DIRS-UNHASHED).

**The executed path and the identified file** (review rounds 3 and 4).

- Every class executes the absolute path the program was found at, so `argv[0]`'s base name is the name the program was asked for. `unshare` executes the path it is given. A multi-call binary, such as the rustup proxy `rustc -> rustup`, therefore runs as the tool asked for in every class.
- The identity digests the file that path resolves to. The kernel reaches the same file through the found path, unless a link on the path is swapped between resolution and exec (RES-G189-HOST-PATH-TOCTOU).
- In round 3, the resolved file was executed and links with another name were refused (`PROGRAM_ALIAS`). Round 4 withdrew that: with `-vV` chosen by the resolved name, a `rustc` proxy run directly was recorded without a version, and a pin accepted another toolchain. `PROGRAM_ALIAS` no longer exists.
- Declared `toolchain_binaries` are identified the same way (path and resolved file). The run does not execute them; the program executes them by the path it names.
- **Toolchain dispatchers are refused** (`TOOLCHAIN_PROXY`, review rounds 5 and 6). The soundness rule does not depend on names. A program or declared binary that is found as, or resolves to, `rustc` is accepted only if two things hold:
  - `<found> --print sysroot`, run exactly as the run executes it (same class, path and declared environment, no request argument, the stage still empty), exits 0;
  - it prints an absolute sysroot whose `bin/rustc` canonicalizes to the file it resolved to.

  A `cargo` is accepted when the `rustc` beside its resolved file passes the same check and the cargo is that sysroot's own `bin/cargo` (G189 review round 7: a dispatcher copied beside a real rustc under the name cargo is refused). That binds cargo to the toolchain directory it comes from, while the compiler cargo invokes at run time stays under RES-G189-TOOLCHAIN-DIRS-UNHASHED.

  A dispatcher is refused whether it is linked, hard-linked or copied under the name `rustc` (rustup's fallback install mode). The rustup proxy is one: it prints the sysroot of the toolchain it selected, whose `bin/rustc` is another file, and its `-vV` answers for whichever toolchain `+toolchain`, the environment or a staged `rust-toolchain.toml` selects.

  An earlier, name-based check gives a clearer refusal before any confinement is set up. It catches a `rustc`/`cargo` link to a file of another name, but it is not the soundness rule.

  A pin therefore hashes a binary that answers for its own sysroot, such as the demonstration's `<sysroot>/bin/rustc`. A wrapper that lies about `--print sysroot` and `-vV` is the requester's own pinned program (RES-G189-TOOLCHAIN-DIRS-UNHASHED).
- A program given as a path that is not absolute and normalized is refused (`REQUEST_MALFORMED`). A relative path would be hashed relative to Atlas's working directory and executed relative to the stage (review finding f1).

A request may pin an identity (`pinned_toolchain`). These cases refuse the run before its program runs:

- an identity that cannot be established (an unreadable binary, or `-vV` failing or printing nothing): `TOOLCHAIN_IDENTITY_ABSENT`;
- an identity that differs from the pin: `TOOLCHAIN_IDENTITY_MISMATCH`. The reason names the first differing field, and the refusal record carries the observed identity.

Refusal is chosen over a recorded mismatch because the contract admits sandbox output as evidence only when the executed identity is pinned: a mismatched run could never become evidence, so it does not run.

### 6. Records

- `SANDBOX_RUN_SCHEMA` becomes `atlas.sandbox-run.v2`. A run gains `toolchain` and a typed `refusal_kind` (`RefusalKind`, one variant per refusal path). A request gains `confinement`, `toolchain_binaries` and `pinned_toolchain`. Every new field is optional on read, so a v1 record still reads (tested).
- No `atlas.sandbox-run` record was committed under `.atlas/evidence`.
- A request without the new fields keeps its v1 request digest, because the canonical text appends lines only when the fields are present. Their values are length-prefixed (`<bytes>:<value>`).
- **Injective identity (review findings f3 and round 3).** On the requests that pass `request_defect`, `canonical_request` is injective up to the order of `inputs`, `outputs`, the toolchain paths and `toolchain_binaries`: two such requests share a canonical text, and so a request digest, only if they differ in nothing but that order. The canonical form sorts the order away, and the order does not change what runs: inputs are staged by path, outputs are digested by path, and toolchain paths are bound and binaries identified in sorted order. The record lists the outputs in request order while the digest is order-free, so consumers compare a run's outputs as a set, by path. Before anything runs, `request_defect` refuses (`REQUEST_MALFORMED`):
  - a control character in the program, an argument, an input or output path, an environment name or value, a toolchain path or a toolchain binary;
  - a single empty argument (`args: [""]`), which joins like `args: []`;
  - `=` or emptiness in an environment name;
  - an input digest that is not `blake3-256:` followed by 64 hex digits (its fixed shape ends the input line unambiguously).

  A newline in an output can therefore no longer forge another request's confinement lines. A multi-line script must be passed as one line (`;`).
- A malformed request's record is identified by the BLAKE3 digest of `atlas.sandbox-request.malformed`, a newline, and its JSON. No canonical text starts with that tag.

Both classes also gain two input and output rules:

- An input that resolves outside the source root through a link is refused (`INPUT_ESCAPES`); inputs are always staged as copies.
- A produced output that is a link, or resolves outside the stage, is recorded as not produced: the host never follows a link the program made.

### 7. CLI

- `atlas-systemizer sandbox probe [--staging D]` reports the confined class's probe result besides the restricted class.
- `atlas-systemizer sandbox run --request <request.json> --source <dir> [--staging <dir>] [--out <file>]` runs one request in the class it asks for. A relative `--staging` is resolved against the current directory, and a missing one is created. It prints the `SandboxRun` with the run's output, and exits non-zero when the request is refused.

## Falsification (`evidence/verification/G189-confined-sandbox.json`)

**Runtime tests.** These run in this container, and the confinement assertions are live: a refusal is accepted as honest only when `unshare` itself cannot enter the namespaces.

- A confined `sh` cannot read any of: `/etc/passwd`, the repository's `Cargo.toml`, a sentinel outside its inputs, the host paths of its own source files, `../`-relative escapes, `/proc/self/root/...`.
- The declared input is readable. The toolchain is readable but neither writable nor remountable.
- Writes to `/`, `/tmp`, the host and `../` fail, and the stage write lands.
- A perl double-chroot escape is refused.
- Links in inputs cannot point outside the source root, and a produced link is not followed.
- The identity is stable across runs, and a mismatched pin and an absent binary are refused.
- With `unshare` pointed at a missing binary (a `HostTools` seam, not global state), the run is `REFUSED` with `FILESYSTEM_CONFINED` `UNENFORCED` and nothing runs.
- Three deliberately broken confinements each fail the probe, and nothing runs: the host run directory shown, the stage hidden, the host `/proc` mounted.
- Every descendant dies with the confined run; in the restricted class, a `setsid` descendant survives.
- The run reads its own `/proc/self/mountinfo`. Its mounts are exactly `/`, `/work`, `/dev/null`, `/proc` and the declared toolchain paths, a regular file among them, all private. The root and toolchain are `ro,nosuid,nodev`, `/work` is `rw,nosuid,nodev`, and `/proc` is `nosuid,nodev,noexec`. A declared file is readable and not writable, which is asked through `access(2)`, so no write is attempted on the host.
- Relative and unnormalized program paths are refused in both classes. The executed file (`/proc/$$/exe`) equals the identity's resolved path.
- A plain v1 `sh` request runs with `$0 = /usr/bin/sh` directly, inside `unshare --net` and confined.
- A rustup-proxy `rustc` or `cargo` is refused `TOOLCHAIN_PROXY` in all three modes: as the program, with `+stable`, with a staged `rust-toolchain.toml`, and as a declared binary. The proxy is refused when linked (by the early name check) and when copied and hard-linked under the names `rustc` and `cargo` (by its sysroot). A real `rustc` linked as `rustc-1.90` is verified and its `-vV` recorded. The toolchain's own `<sysroot>/bin/rustc` runs and pins in all three modes, and another sysroot's binary is refused.
- A declared link is a link in the root: `alias/../marker` climbs as on the host. A link whose target is undeclared is refused, and so is a declaration passing through a link.
- The f3 forgery is refused under its own digest, a `:` in a pinned path is unambiguous, and the v1 canonical text is pinned verbatim.
- A missing staging directory is created, a non-normalized one works, and one that cannot be created is a host error.
- `argv[0]`'s base name is the link name in all three modes. The test observes it through links to the host shell running `sh -c 'echo ${0##*/}'`.
- `-vV` is asked of a program found as `rustc`.
- In the confined class, a found path or a resolved target outside the declared toolchain paths is refused.
- A declared link that resolves into `/run` is refused.
- `args: []` and `args: [""]` no longer share an identity.
- Every loader name and every socket directory is refused, each with its own reason.

**Demonstration.**

- `rustc` 1.90.0 compiles and links a declared `main.rs` inside the confinement. The output digest is recorded, and is identical across two runs.
- `include_str!("/etc/hostname")` and `include_str!("/home/user/Atlas-Studio/Cargo.toml")` fail inside the confinement (`couldn't read ... No such file or directory`). The same requests without `confinement` compile.
- The same compile pinned to the 1.90.0 identity is refused when run with the stable toolchain, and when the pinned `cc` digest is altered.
- The evidence records the exact source bytes and digests and every full request. Every request digest is recomputed by an independent implementation and matches.

**Mutants.** On the final code, 55 hand mutants; 51 killed. Four survived:

- the probe's stderr match: redundant with the positive control under every tested failure;
- `--propagation unchanged`: equivalent on this host, which has no shared mounts;
- a probe with an empty environment: path resolution by `unshare` does not read it;
- a remount without `nosuid`: the tmpfs is created `nosuid`.

Earlier rounds: round 1 ran 22 mutants and killed 21; one of them wrote to the host's `/usr/bin` while live, and that write probe is gone. Rounds 2 to 5 killed 32 of 37, 39 of 44, 40 of 44 and 45 of 50.

## Consequences

**M22 now has a mechanism.** A confined run with per-run toolchain identity exists and is probed per run. It is demonstrated here on a real `rustc` compile and link. This ADR does not edit the roadmap.

**What the confined class is not:**

- **No seccomp filter.** The program can make every system call, so a kernel defect is outside the model.
- **No cgroup limits.** There is no memory, CPU or pids limit beyond the timeout, and a fork bomb inside the pid namespace is bounded only by the host.
- **Network denial** is the existing `--net` mechanism, applied when the request denies the network. Without it, the host network and abstract unix sockets are reachable.
- **The caller's kernel uid.** The program has the caller's kernel uid (root in this container): no capability, but owner rights on everything it can see. Every visible path is read-only except `/work`.
- **The host mount tree** still exists in the run's mount namespace below the chroot, unreachable without a capability.
- **Tested environment.** Tested only as root in a container whose kernel permits unprivileged user namespaces. On a host that refuses them the class refuses, and claims nothing.
- **Helpers are not identified.** The helper binaries (`unshare`, `sh`, `mount`, `mkdir`, `ln`) are trusted host binaries, not identified in the record. Toolchain directories are not hashed, only the program and declared binaries. For `rustc`, `-vV` (commit hash) stands for the driver library unless it is declared. For `cargo`, `-vV` identifies cargo, not the compiler it invokes (`RUSTC`, `RUSTC_WRAPPER` and `PATH` select that). A wrapper that answers `--print sysroot` and `-vV` falsely is accepted as what it is: the requester's own program, pinned by its digest.
- **The loader environment refusal is a list.**
- **Sockets in declared directories.** A socket inside a declared toolchain directory is connectable. Only the known socket directories are refused.
- **TOCTOU.** A host path, or a link on the program's path, can be swapped between the check (or the resolution) and the bind (or the exec).

**Next steps, unclaimed.** `runtime::verification` command evidence still runs with ambient authority. Verification evidence produced inside the sandbox, and M16 (the confined delegated build, NA-CONFINED-DELEGATED-BUILD), are the next steps. `DEBT-SANDBOXED_EXECUTION` advances, but builds and tests of Atlas itself do not yet run confined.

This is a probed, falsified mechanism on this host, not a security certification.
