#!/usr/bin/env node
// Unified repository-layout gate.
//
// Keep docs topology and source topology fail-closed behind the PR policy job that already
// exists on main. This avoids a second workflow/authority surface and guarantees that the tiny
// docs allowlist is checked whenever the hard repository-topology gate runs.
import '../docs/check-layout.mjs'
import './verify-repository-topology-core.mjs'
