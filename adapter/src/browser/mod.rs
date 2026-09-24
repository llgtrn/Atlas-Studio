//! Sandboxed browser instrument for Creator Fabric observation (ADR 0011).
//!
//! External boundary: Playwright driving headless Chromium, both recorded in every report's
//! `instrument`. Atlas owns what is measured and how it is interpreted; the browser is a measuring
//! instrument, the way a compiler binary is an oracle. Scope is deliberately narrow: a local
//! fixture file only (`LOCAL_FIXTURE` authorization), a fresh browser context per viewport, and
//! every network request other than the subject itself aborted -- no authentication, no cookies,
//! no evasion of any access control. Returns the instrument's raw JSON; parsing into typed
//! observations belongs to `runtime::visual` (this crate keeps JSON parsing out of production).

use atlas_core::visual::OBSERVED_STYLE_PROPERTIES;
use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
};

const OBSERVATION_SCRIPT: &str = include_str!("observe.js");

/// Upper bound on elements recorded per viewport; beyond it the report is marked truncated.
pub const OBSERVATION_ELEMENT_LIMIT: usize = 5000;

/// The Playwright module directory: `ATLAS_PLAYWRIGHT_MODULE`, else `$(npm root -g)/playwright`.
pub fn playwright_module() -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os("ATLAS_PLAYWRIGHT_MODULE") {
        let path = PathBuf::from(explicit);
        return path.join("package.json").is_file().then_some(path);
    }
    let output = Command::new("npm").args(["root", "-g"]).output().ok()?;
    let root = String::from_utf8(output.stdout).ok()?;
    let path = PathBuf::from(root.trim()).join("playwright");
    path.join("package.json").is_file().then_some(path)
}

/// Observes `fixture` (a regular local file) at each `(width, height)` viewport and returns the
/// instrument's JSON document.
pub fn observe_fixture_raw(fixture: &Path, viewports: &[(u32, u32)]) -> io::Result<String> {
    run_instrument(fixture, viewports, "layout")
}

/// Applies stimuli (hover, click, click twice, keyboard focus) to every interactive candidate of
/// `fixture` at one viewport, each in a fresh context, and returns the observed state changes
/// after all animations settle (ADR 0015).
pub fn interact_fixture_raw(fixture: &Path, viewport: (u32, u32)) -> io::Result<String> {
    run_instrument(fixture, &[viewport], "interact")
}

fn run_instrument(fixture: &Path, viewports: &[(u32, u32)], mode: &str) -> io::Result<String> {
    let fixture = fixture.canonicalize()?;
    if !fs_is_regular_file(&fixture) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "observation subject must be a regular local file",
        ));
    }
    if viewports.is_empty() || viewports.iter().any(|(w, h)| *w == 0 || *h == 0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "at least one non-empty viewport is required",
        ));
    }
    let module = playwright_module().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "browser instrument unavailable: no Playwright module (set ATLAS_PLAYWRIGHT_MODULE)",
        )
    })?;
    let url = format!("file://{}", fixture.to_string_lossy());
    let viewport_list = viewports
        .iter()
        .map(|(w, h)| format!("{w}x{h}"))
        .collect::<Vec<_>>()
        .join(",");
    let properties = format!(
        "[{}]",
        OBSERVED_STYLE_PROPERTIES
            .iter()
            .map(|p| format!("\"{p}\""))
            .collect::<Vec<_>>()
            .join(",")
    );
    let output = Command::new("node")
        .arg("-e")
        .arg(OBSERVATION_SCRIPT)
        .env("ATLAS_PLAYWRIGHT_MODULE", &module)
        .env("ATLAS_OBSERVE_URL", &url)
        .env("ATLAS_OBSERVE_MODE", mode)
        .env("ATLAS_OBSERVE_VIEWPORTS", viewport_list)
        .env("ATLAS_OBSERVE_PROPERTIES", properties)
        .env(
            "ATLAS_OBSERVE_ELEMENT_LIMIT",
            OBSERVATION_ELEMENT_LIMIT.to_string(),
        )
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "browser instrument failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    String::from_utf8(output.stdout).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn fs_is_regular_file(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|meta| meta.is_file())
}
