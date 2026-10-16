# Codec report output preservation

The existing CLI `atlasx codec --decode OBJECT --out REPORT` formerly used a shared truncating
report writer. Choosing the input path, a hard-link alias or a symlink alias could replace the
historical artifact with JSON. A prior report could also be silently overwritten.

Only this codec decode caller now uses create-or-identical-regular-replay output. Existing
paths are never opened for writing. Conflicts, input aliases and symlinks refuse; identical
regular-file reports replay without mutation. Parent creation and typed report JSON remain
compatible. Invalid objects still produce the complete refusal report and existing refusal exit
when report creation/replay succeeds. Other commands retain their existing report behavior.
Concurrent path replacement is outside this source filesystem model; this is not a new sandbox,
authority grant or durable Chronica effect protocol.

Three exact CLI caller tests exercise input/relative/hard-link/symlink aliases, different-length
and equal-length conflicts, exact replay, missing parent creation, the historical SR1 round trip
and the full typed refusal report/exit on replay. They passed with zero failures or ignored
cases. The [fresh compiled-process observation](source-72ded57-process.json) records fourteen
actual CLI process cases against source `72ded57a1fb9227136f7b34df92c559cc5c19b83`,
including typed stdout/refusal reports and alias/conflict/replay checks. Every case passed; input
and prior-report bytes remained unchanged. The receipt pins source/tree, CLI binary, source file
and fixture bytes. No complete workspace or native hardware pass is claimed.

This bounded source fix supports evidence preservation in N04/N05 and J2/J8; it proves none of
the physical native journey, Chronica adoption, rights or source-extinction obligations. Existing
formats, immutable payloads, report wire fields, saved ArtifactIds and historical records remain
unchanged. The earlier compatibility receipt retains its own source/candidate scope and BLOCKED
full-self verdict; this change does not promote it into current full-scope admission.
