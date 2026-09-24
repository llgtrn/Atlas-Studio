//! Bounded donor working set (ADR 0021): donor source is temporary working material held inside a
//! measured disk budget, never an archive.
//!
//! DISCOVER MANY -> QUEUE MANY -> MATERIALIZE FEW -> UNDERSTAND -> ABSORB -> VERIFY -> DELETE
//! SOURCE -> RECLAIM -> NEXT DONOR.
//!
//! Storage state is orthogonal to decision state (`decision = ABSORB_NOW` with
//! `storage = MATERIALIZED_SLICE` is ordinary). Everything here is pure and integer-exact: the
//! runtime measures the filesystem, this module decides.

use serde::{Deserialize, Serialize};

/// Where a donor's source physically is -- independent of what Atlas has decided about it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StorageState {
    /// Known from remote metadata only (URL, head, tree, license); nothing local.
    RemoteDiscovered,
    /// Selected for the working set, not yet materialized.
    Queued,
    /// Materialization in progress.
    Materializing,
    /// Only the mechanism-relevant paths are local.
    MaterializedSlice,
    /// A (shallow or full) checkout is local.
    MaterializedCheckout,
    Censusing,
    Absorbing,
    /// The absorbed mechanism is native and verified; source still local.
    VerifiedNative,
    /// Nothing still needs the local source; deletion is the next step.
    SourceDeletable,
    /// Source was materialized and has been physically deleted.
    SourceDeleted,
    /// Source was never local (remote census only): no deletion ever happened or is owed.
    SourceNeverMaterialized,
}

impl StorageState {
    pub const ALL: [StorageState; 11] = [
        Self::RemoteDiscovered,
        Self::Queued,
        Self::Materializing,
        Self::MaterializedSlice,
        Self::MaterializedCheckout,
        Self::Censusing,
        Self::Absorbing,
        Self::VerifiedNative,
        Self::SourceDeletable,
        Self::SourceDeleted,
        Self::SourceNeverMaterialized,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::RemoteDiscovered => "REMOTE_DISCOVERED",
            Self::Queued => "QUEUED",
            Self::Materializing => "MATERIALIZING",
            Self::MaterializedSlice => "MATERIALIZED_SLICE",
            Self::MaterializedCheckout => "MATERIALIZED_CHECKOUT",
            Self::Censusing => "CENSUSING",
            Self::Absorbing => "ABSORBING",
            Self::VerifiedNative => "VERIFIED_NATIVE",
            Self::SourceDeletable => "SOURCE_DELETABLE",
            Self::SourceDeleted => "SOURCE_DELETED",
            Self::SourceNeverMaterialized => "SOURCE_NEVER_MATERIALIZED",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|state| state.as_str() == text)
    }

    /// Whether local source must exist (`Some(true)`), must not (`Some(false)`), or may be
    /// partial either way (`None`, only while materializing).
    pub fn source_present(self) -> Option<bool> {
        match self {
            Self::Materializing => None,
            Self::MaterializedSlice
            | Self::MaterializedCheckout
            | Self::Censusing
            | Self::Absorbing
            | Self::VerifiedNative
            | Self::SourceDeletable => Some(true),
            Self::RemoteDiscovered
            | Self::Queued
            | Self::SourceDeleted
            | Self::SourceNeverMaterialized => Some(false),
        }
    }

    /// Active work on local source that should finish before the working set expands.
    pub fn in_progress(self) -> bool {
        matches!(
            self,
            Self::Materializing | Self::Censusing | Self::Absorbing
        )
    }
}

/// Materialization strategies, cheapest first. The cheapest one that can produce the required
/// evidence is chosen (`cheapest_sufficient_mode`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MaterializationMode {
    RemoteMetadata,
    FetchedFileSet,
    SparseCheckout,
    ShallowCheckout,
    FullCheckout,
}

impl MaterializationMode {
    pub fn is_local(self) -> bool {
        self != Self::RemoteMetadata
    }
}

/// What the census of a donor needs to establish its claims.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceNeeds {
    /// Reading a known, bounded set of files.
    pub reads_files: bool,
    /// A directory subtree (mechanism slice) must be read coherently.
    pub reads_subtree: bool,
    /// Compilation, execution, profiling, mutation, generated files or whole-repo resolution.
    pub executes: bool,
    /// Commit history (blame, bisect, provenance across revisions).
    pub needs_history: bool,
}

/// The cheapest materialization that can produce `needs`. A full checkout is chosen only for
/// history, which is its explicit evidence reason.
pub fn cheapest_sufficient_mode(needs: EvidenceNeeds) -> MaterializationMode {
    if needs.needs_history {
        MaterializationMode::FullCheckout
    } else if needs.executes {
        MaterializationMode::ShallowCheckout
    } else if needs.reads_subtree {
        MaterializationMode::SparseCheckout
    } else if needs.reads_files {
        MaterializationMode::FetchedFileSet
    } else {
        MaterializationMode::RemoteMetadata
    }
}

/// A filesystem measurement. `capacity_bytes` is used + available (the `df` Use% basis), so a
/// per-session allowance or reserved blocks never inflate it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiskMeasurement {
    pub capacity_bytes: u64,
    pub available_bytes: u64,
}

impl DiskMeasurement {
    /// Free fraction in parts per thousand, rounded down.
    pub fn free_permille(self) -> u64 {
        if self.capacity_bytes == 0 {
            return 0;
        }
        (u128::from(self.available_bytes) * 1000 / u128::from(self.capacity_bytes)) as u64
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Pressure {
    /// New materialization allowed.
    Healthy,
    /// Finish current donors (and reclaim deletable source) before expanding.
    Pressure,
    /// No new large donor; prioritize extinction and cleanup.
    HighPressure,
    /// Materialization hard stop.
    Critical,
}

/// Configurable policy: ratios with absolute floors, never one machine's numbers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoragePolicy {
    /// Free fraction (permille) at or above which the disk is `Healthy`.
    pub healthy_min_free_permille: u64,
    pub pressure_min_free_permille: u64,
    pub high_pressure_min_free_permille: u64,
    /// Reserved for builds/tests, as a fraction of available space with an absolute floor.
    pub build_headroom_permille: u64,
    pub build_headroom_min_bytes: u64,
    /// Never spent on donors, as a fraction of capacity with an absolute floor.
    pub safety_reserve_permille: u64,
    pub safety_reserve_min_bytes: u64,
    /// Upper bound on all local donor source together, as a fraction of capacity.
    pub max_working_set_permille: u64,
    /// A donor at or above this estimated footprint is "large".
    pub large_donor_bytes: u64,
}

impl Default for StoragePolicy {
    fn default() -> Self {
        const GIB: u64 = 1 << 30;
        Self {
            healthy_min_free_permille: 400,
            pressure_min_free_permille: 200,
            high_pressure_min_free_permille: 100,
            build_headroom_permille: 250,
            build_headroom_min_bytes: 2 * GIB,
            safety_reserve_permille: 50,
            safety_reserve_min_bytes: GIB,
            max_working_set_permille: 250,
            large_donor_bytes: 256 << 20,
        }
    }
}

impl StoragePolicy {
    pub fn validate(&self) -> Result<(), String> {
        if !(self.healthy_min_free_permille > self.pressure_min_free_permille
            && self.pressure_min_free_permille > self.high_pressure_min_free_permille
            && self.healthy_min_free_permille <= 1000)
        {
            return Err(
                "watermarks must satisfy 1000 >= healthy > pressure > high_pressure".into(),
            );
        }
        if self.build_headroom_permille > 1000
            || self.safety_reserve_permille > 1000
            || self.max_working_set_permille > 1000
        {
            return Err("fractions are permille and cannot exceed 1000".into());
        }
        Ok(())
    }

    pub fn pressure(&self, disk: DiskMeasurement) -> Pressure {
        let free = disk.free_permille();
        if free >= self.healthy_min_free_permille {
            Pressure::Healthy
        } else if free >= self.pressure_min_free_permille {
            Pressure::Pressure
        } else if free >= self.high_pressure_min_free_permille {
            Pressure::HighPressure
        } else {
            Pressure::Critical
        }
    }

    fn permille_of(value: u64, permille: u64) -> u64 {
        (u128::from(value) * u128::from(permille) / 1000) as u64
    }

    pub fn build_headroom(&self, disk: DiskMeasurement) -> u64 {
        Self::permille_of(disk.available_bytes, self.build_headroom_permille)
            .max(self.build_headroom_min_bytes)
    }

    pub fn safety_reserve(&self, disk: DiskMeasurement) -> u64 {
        Self::permille_of(disk.capacity_bytes, self.safety_reserve_permille)
            .max(self.safety_reserve_min_bytes)
    }

    /// TOTAL_FREE - BUILD_HEADROOM - SAFETY_RESERVE: what new donor material may occupy now.
    pub fn donor_budget(&self, disk: DiskMeasurement) -> u64 {
        disk.available_bytes
            .saturating_sub(self.build_headroom(disk))
            .saturating_sub(self.safety_reserve(disk))
    }

    pub fn max_working_set(&self, disk: DiskMeasurement) -> u64 {
        Self::permille_of(disk.capacity_bytes, self.max_working_set_permille)
    }
}

/// A donor currently holding (or about to hold) local source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkingSetEntry {
    pub donor: String,
    pub state: StorageState,
    /// Measured on disk (source plus any build output under Atlas's control).
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdmissionRequest {
    pub donor: String,
    pub mode: MaterializationMode,
    pub estimated_source_bytes: u64,
    /// Build/generated/test output the census is expected to create (build amplification).
    pub estimated_build_bytes: u64,
}

impl AdmissionRequest {
    pub fn footprint(&self) -> u64 {
        self.estimated_source_bytes
            .saturating_add(self.estimated_build_bytes)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "reason", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdmissionRefusal {
    InvalidPolicy { detail: String },
    CriticalDisk,
    LargeDonorUnderHighPressure,
    FinishCurrentDonorsFirst { in_progress: Vec<String> },
    ReclaimDeletableSourceFirst { deletable: Vec<String> },
    AlreadyInWorkingSet,
    ExceedsDonorBudget { footprint: u64, budget: u64 },
    ExceedsWorkingSetCap { would_hold: u64, cap: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdmissionDecision {
    pub donor: String,
    pub pressure: Pressure,
    pub donor_budget_bytes: u64,
    pub working_set_bytes: u64,
    pub working_set_cap_bytes: u64,
    /// `None` = admitted. Remote-only requests are always admitted: they occupy no disk.
    pub refusal: Option<AdmissionRefusal>,
}

impl AdmissionDecision {
    pub fn admitted(&self) -> bool {
        self.refusal.is_none()
    }
}

/// Decides whether `request` may enter the local working set under `policy` and the measured
/// disk. `working_set` lists every donor whose source is (or is becoming) local.
pub fn decide_admission(
    policy: &StoragePolicy,
    disk: DiskMeasurement,
    working_set: &[WorkingSetEntry],
    request: &AdmissionRequest,
) -> AdmissionDecision {
    let pressure = policy.pressure(disk);
    let donor_budget_bytes = policy.donor_budget(disk);
    let working_set_bytes = working_set
        .iter()
        .filter(|e| e.state.source_present() != Some(false))
        .map(|e| e.bytes)
        .fold(0u64, u64::saturating_add);
    let working_set_cap_bytes = policy.max_working_set(disk);
    let refusal = refusal(policy, pressure, working_set, request).or_else(|| {
        if !request.mode.is_local() {
            return None;
        }
        let footprint = request.footprint();
        if footprint > donor_budget_bytes {
            return Some(AdmissionRefusal::ExceedsDonorBudget {
                footprint,
                budget: donor_budget_bytes,
            });
        }
        let would_hold = working_set_bytes.saturating_add(footprint);
        (would_hold > working_set_cap_bytes).then_some(AdmissionRefusal::ExceedsWorkingSetCap {
            would_hold,
            cap: working_set_cap_bytes,
        })
    });
    AdmissionDecision {
        donor: request.donor.clone(),
        pressure,
        donor_budget_bytes,
        working_set_bytes,
        working_set_cap_bytes,
        refusal,
    }
}

fn refusal(
    policy: &StoragePolicy,
    pressure: Pressure,
    working_set: &[WorkingSetEntry],
    request: &AdmissionRequest,
) -> Option<AdmissionRefusal> {
    if let Err(detail) = policy.validate() {
        return Some(AdmissionRefusal::InvalidPolicy { detail });
    }
    if working_set
        .iter()
        .any(|e| e.donor == request.donor && e.state.source_present() != Some(false))
    {
        return Some(AdmissionRefusal::AlreadyInWorkingSet);
    }
    if !request.mode.is_local() {
        return None;
    }
    let names = |keep: fn(StorageState) -> bool| -> Vec<String> {
        working_set
            .iter()
            .filter(|e| keep(e.state))
            .map(|e| e.donor.clone())
            .collect()
    };
    match pressure {
        Pressure::Healthy => None,
        Pressure::Pressure => {
            let in_progress = names(StorageState::in_progress);
            let deletable = names(|s| s == StorageState::SourceDeletable);
            if !in_progress.is_empty() {
                Some(AdmissionRefusal::FinishCurrentDonorsFirst { in_progress })
            } else if !deletable.is_empty() {
                Some(AdmissionRefusal::ReclaimDeletableSourceFirst { deletable })
            } else {
                None
            }
        }
        Pressure::HighPressure => (request.footprint() >= policy.large_donor_bytes)
            .then_some(AdmissionRefusal::LargeDonorUnderHighPressure),
        Pressure::Critical => Some(AdmissionRefusal::CriticalDisk),
    }
}

/// How a dependency of a donor terminates. Every dependency must be accounted; only
/// `SourceRequired` is ever materialized (TRANSITIVE_CLOSURE_REQUIRED is about accounting, not
/// cloning).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DependencyTerminal {
    /// Its source implements part of the mechanism being absorbed.
    SourceRequired,
    /// OS, ISA, libc, kernel interface.
    SystemBoundary,
    /// Compiler, build tool, package manager.
    Toolchain,
    /// Network service or hosted API.
    Service,
    /// Replaceable compute provider (model host, media vendor).
    ExternalProvider,
    /// Already implemented natively in Atlas.
    AlreadyNative,
    /// Accounted by remote identity; its source is not needed.
    ReferenceOnly,
}

impl DependencyTerminal {
    pub fn requires_materialization(self) -> bool {
        self == Self::SourceRequired
    }
}

/// One donor as recorded, with the physically observed presence of its local source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageRecord {
    pub donor: String,
    pub state: StorageState,
    pub source_present: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "violation", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StorageViolation {
    /// Recorded deleted / never local / not yet local, but source exists.
    SourcePresentButRecordedAbsent { donor: String, state: StorageState },
    /// Recorded local, but no source exists.
    SourceAbsentButRecordedPresent { donor: String, state: StorageState },
    /// A local donor directory that no record owns.
    UnownedCheckout { directory: String },
}

/// Storage integrity: every record agrees with the disk, and every local donor directory is owned
/// by a record. `directories` are the donor directories found on disk; `owner_of` maps a
/// directory to the donor that owns it, if any.
pub fn check_integrity(
    records: &[StorageRecord],
    directories: &[String],
    owner_of: impl Fn(&str) -> Option<String>,
) -> Vec<StorageViolation> {
    let mut violations = Vec::new();
    for record in records {
        match (record.state.source_present(), record.source_present) {
            (Some(false), true) => {
                violations.push(StorageViolation::SourcePresentButRecordedAbsent {
                    donor: record.donor.clone(),
                    state: record.state,
                })
            }
            (Some(true), false) => {
                violations.push(StorageViolation::SourceAbsentButRecordedPresent {
                    donor: record.donor.clone(),
                    state: record.state,
                })
            }
            _ => {}
        }
    }
    for directory in directories {
        let owned = owner_of(directory).is_some_and(|donor| {
            records
                .iter()
                .any(|r| r.donor == donor && r.state.source_present() != Some(false))
        });
        if !owned {
            violations.push(StorageViolation::UnownedCheckout {
                directory: directory.clone(),
            });
        }
    }
    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1 << 30;
    const MIB: u64 = 1 << 20;

    fn disk(capacity_gib: u64, available_gib: u64) -> DiskMeasurement {
        DiskMeasurement {
            capacity_bytes: capacity_gib * GIB,
            available_bytes: available_gib * GIB,
        }
    }

    fn request(
        donor: &str,
        mode: MaterializationMode,
        source: u64,
        build: u64,
    ) -> AdmissionRequest {
        AdmissionRequest {
            donor: donor.into(),
            mode,
            estimated_source_bytes: source,
            estimated_build_bytes: build,
        }
    }

    fn entry(donor: &str, state: StorageState, bytes: u64) -> WorkingSetEntry {
        WorkingSetEntry {
            donor: donor.into(),
            state,
            bytes,
        }
    }

    #[test]
    fn watermarks_partition_the_free_fraction() {
        let policy = StoragePolicy::default();
        assert_eq!(policy.pressure(disk(100, 40)), Pressure::Healthy);
        assert_eq!(policy.pressure(disk(100, 39)), Pressure::Pressure);
        assert_eq!(policy.pressure(disk(100, 20)), Pressure::Pressure);
        assert_eq!(policy.pressure(disk(100, 19)), Pressure::HighPressure);
        assert_eq!(policy.pressure(disk(100, 10)), Pressure::HighPressure);
        assert_eq!(policy.pressure(disk(100, 9)), Pressure::Critical);
        assert_eq!(policy.pressure(disk(0, 0)), Pressure::Critical);
    }

    #[test]
    fn the_budget_subtracts_build_headroom_and_safety_reserve() {
        let policy = StoragePolicy::default();
        // 100 GiB capacity, 60 free: headroom max(15, 2) = 15, reserve max(5, 1) = 5 -> 40.
        assert_eq!(policy.donor_budget(disk(100, 60)), 40 * GIB);
        // Floors dominate on a small disk: 4 GiB capacity, 3 free: headroom 2, reserve 1 -> 0.
        assert_eq!(policy.donor_budget(disk(4, 3)), 0);
        assert_eq!(policy.max_working_set(disk(100, 60)), 25 * GIB);
    }

    #[test]
    fn critical_disk_refuses_every_local_materialization_but_not_remote_discovery() {
        let policy = StoragePolicy::default();
        let d = disk(100, 5);
        let local = decide_admission(
            &policy,
            d,
            &[],
            &request("x", MaterializationMode::FetchedFileSet, MIB, 0),
        );
        assert_eq!(local.refusal, Some(AdmissionRefusal::CriticalDisk));
        let remote = decide_admission(
            &policy,
            d,
            &[],
            &request("x", MaterializationMode::RemoteMetadata, 0, 0),
        );
        assert!(remote.admitted());
    }

    #[test]
    fn high_pressure_admits_only_small_donors() {
        let policy = StoragePolicy::default();
        let d = disk(100, 15);
        let large = decide_admission(
            &policy,
            d,
            &[],
            &request(
                "big",
                MaterializationMode::SparseCheckout,
                200 * MIB,
                100 * MIB,
            ),
        );
        assert_eq!(
            large.refusal,
            Some(AdmissionRefusal::LargeDonorUnderHighPressure)
        );
        let small = decide_admission(
            &policy,
            d,
            &[],
            &request("tiny", MaterializationMode::FetchedFileSet, MIB, 0),
        );
        assert!(small.admitted(), "{small:?}");
    }

    #[test]
    fn pressure_finishes_current_donors_and_reclaims_before_expanding() {
        let policy = StoragePolicy::default();
        let d = disk(100, 30);
        let new = request("next", MaterializationMode::SparseCheckout, MIB, 0);
        let busy = [entry("a", StorageState::Censusing, GIB)];
        assert_eq!(
            decide_admission(&policy, d, &busy, &new).refusal,
            Some(AdmissionRefusal::FinishCurrentDonorsFirst {
                in_progress: vec!["a".into()]
            })
        );
        let done = [entry("a", StorageState::SourceDeletable, GIB)];
        assert_eq!(
            decide_admission(&policy, d, &done, &new).refusal,
            Some(AdmissionRefusal::ReclaimDeletableSourceFirst {
                deletable: vec!["a".into()]
            })
        );
        let blocked = [entry("a", StorageState::MaterializedCheckout, GIB)];
        assert!(decide_admission(&policy, d, &blocked, &new).admitted());
        // Healthy disk: several tiny independent donors may be in progress together.
        assert!(decide_admission(&policy, disk(100, 60), &busy, &new).admitted());
    }

    #[test]
    fn the_working_set_never_exceeds_its_budget_or_cap() {
        let policy = StoragePolicy::default();
        let d = disk(100, 60); // budget 40 GiB, cap 25 GiB
        let over_budget = request(
            "x",
            MaterializationMode::ShallowCheckout,
            30 * GIB,
            11 * GIB,
        );
        assert_eq!(
            decide_admission(&policy, d, &[], &over_budget).refusal,
            Some(AdmissionRefusal::ExceedsDonorBudget {
                footprint: 41 * GIB,
                budget: 40 * GIB
            })
        );
        let held = [
            entry("a", StorageState::MaterializedCheckout, 20 * GIB),
            entry("gone", StorageState::SourceDeleted, 30 * GIB),
        ];
        let fits_budget = request("y", MaterializationMode::ShallowCheckout, 4 * GIB, 2 * GIB);
        let decision = decide_admission(&policy, d, &held, &fits_budget);
        assert_eq!(
            decision.working_set_bytes,
            20 * GIB,
            "deleted source does not count"
        );
        assert_eq!(
            decision.refusal,
            Some(AdmissionRefusal::ExceedsWorkingSetCap {
                would_hold: 26 * GIB,
                cap: 25 * GIB
            })
        );
        let exactly = request("z", MaterializationMode::ShallowCheckout, 3 * GIB, 2 * GIB);
        assert!(decide_admission(&policy, d, &held, &exactly).admitted());
        let again = request("a", MaterializationMode::SparseCheckout, MIB, 0);
        assert_eq!(
            decide_admission(&policy, d, &held, &again).refusal,
            Some(AdmissionRefusal::AlreadyInWorkingSet)
        );
    }

    #[test]
    fn an_invalid_policy_refuses_instead_of_guessing() {
        let policy = StoragePolicy {
            pressure_min_free_permille: 500,
            ..StoragePolicy::default()
        };
        let decision = decide_admission(
            &policy,
            disk(100, 90),
            &[],
            &request("x", MaterializationMode::FetchedFileSet, 1, 0),
        );
        assert!(matches!(
            decision.refusal,
            Some(AdmissionRefusal::InvalidPolicy { .. })
        ));
    }

    #[test]
    fn the_cheapest_sufficient_mode_is_chosen() {
        use MaterializationMode::*;
        let needs = |reads_files, reads_subtree, executes, needs_history| EvidenceNeeds {
            reads_files,
            reads_subtree,
            executes,
            needs_history,
        };
        assert_eq!(
            cheapest_sufficient_mode(needs(false, false, false, false)),
            RemoteMetadata
        );
        assert_eq!(
            cheapest_sufficient_mode(needs(true, false, false, false)),
            FetchedFileSet
        );
        assert_eq!(
            cheapest_sufficient_mode(needs(true, true, false, false)),
            SparseCheckout
        );
        assert_eq!(
            cheapest_sufficient_mode(needs(true, true, true, false)),
            ShallowCheckout
        );
        assert_eq!(
            cheapest_sufficient_mode(needs(false, false, false, true)),
            FullCheckout
        );
        assert!(RemoteMetadata < FetchedFileSet && ShallowCheckout < FullCheckout);
    }

    #[test]
    fn an_accounted_dependency_is_not_automatically_materialized() {
        use DependencyTerminal::*;
        for terminal in [
            SystemBoundary,
            Toolchain,
            Service,
            ExternalProvider,
            AlreadyNative,
            ReferenceOnly,
        ] {
            assert!(!terminal.requires_materialization(), "{terminal:?}");
        }
        assert!(SourceRequired.requires_materialization());
    }

    #[test]
    fn integrity_catches_every_disagreement_between_record_and_disk() {
        let record = |donor: &str, state, source_present| StorageRecord {
            donor: donor.into(),
            state,
            source_present,
        };
        let records = [
            record("extinct-but-present", StorageState::SourceDeleted, true),
            record(
                "never-but-present",
                StorageState::SourceNeverMaterialized,
                true,
            ),
            record(
                "materialized-but-absent",
                StorageState::MaterializedSlice,
                false,
            ),
            record("fine-present", StorageState::MaterializedCheckout, true),
            record("fine-absent", StorageState::SourceDeleted, false),
            record("in-flight", StorageState::Materializing, false),
        ];
        let dirs = vec![
            "fine-present".to_owned(),
            "orphan".to_owned(),
            "fine-absent".to_owned(),
        ];
        let violations =
            check_integrity(&records, &dirs, |d| (d != "orphan").then(|| d.to_owned()));
        assert_eq!(
            violations,
            vec![
                StorageViolation::SourcePresentButRecordedAbsent {
                    donor: "extinct-but-present".into(),
                    state: StorageState::SourceDeleted
                },
                StorageViolation::SourcePresentButRecordedAbsent {
                    donor: "never-but-present".into(),
                    state: StorageState::SourceNeverMaterialized
                },
                StorageViolation::SourceAbsentButRecordedPresent {
                    donor: "materialized-but-absent".into(),
                    state: StorageState::MaterializedSlice
                },
                StorageViolation::UnownedCheckout {
                    directory: "orphan".into()
                },
                // A directory owned by a donor recorded as deleted is not owned.
                StorageViolation::UnownedCheckout {
                    directory: "fine-absent".into()
                },
            ]
        );
    }

    #[test]
    fn every_state_round_trips_and_has_a_presence_rule() {
        for state in StorageState::ALL {
            assert_eq!(StorageState::parse(state.as_str()), Some(state));
        }
        assert_eq!(StorageState::parse("CLONED"), None);
        let local: Vec<_> = StorageState::ALL
            .into_iter()
            .filter(|s| s.source_present() == Some(true))
            .collect();
        assert_eq!(local.len(), 6);
        assert!(!local.contains(&StorageState::SourceNeverMaterialized));
    }
}
