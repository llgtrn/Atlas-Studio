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

/// The in-page layout measurement every backend evaluates (engine-neutral web-platform code).
const MEASURE_LAYOUT_SCRIPT: &str = include_str!("measure_layout.js");
const OBSERVATION_SCRIPT: &str = include_str!("observe.js");
const WEBDRIVER_SCRIPT: &str = include_str!("webdriver.js");

/// Blink lays out in 1/64 px units (LayoutUnit).
pub const BLINK_LAYOUT_RESOLUTION_PX: f64 = 1.0 / 64.0;
/// LibWeb (Ladybird) `CSSPixels`: 6 fractional bits (source-backed, see `ladybird()`).
pub const LIBWEB_LAYOUT_RESOLUTION_PX: f64 = 1.0 / 64.0;

/// What one instrument can do. An unsupported operation is refused explicitly
/// (`ErrorKind::Unsupported`), never attempted silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstrumentCapabilities {
    pub layout: bool,
    pub interaction: bool,
    pub motion_sampling: bool,
    /// Every request other than the subject (including other local files) is blocked.
    pub network_isolation: bool,
    /// A fresh browser context/session per viewport.
    pub fresh_context_per_viewport: bool,
}

/// A replaceable browser engine used as a measuring instrument (ADR 0022). Backends lower into
/// one raw JSON contract (`engine`, `engine_version`, `driver`, `driver_version`, `network`,
/// `layout_resolution_px`, then `viewports` / `interactions` / `motions`); Atlas semantics are
/// derived above this boundary, never inside it.
pub trait BrowserInstrument {
    /// Registry id (`CREATOR-INSTRUMENTS.toml`).
    fn id(&self) -> &str;
    fn capabilities(&self) -> InstrumentCapabilities;
    /// `Err(reason)` when the instrument cannot run here (missing binary, version mismatch).
    fn availability(&self) -> Result<(), String>;
    fn observe_layout(&self, fixture: &Path, viewports: &[(u32, u32)]) -> io::Result<String>;
    fn observe_interactions(&self, fixture: &Path, viewport: (u32, u32)) -> io::Result<String> {
        let _ = (fixture, viewport);
        Err(unsupported(self.id(), "interaction"))
    }
    fn observe_motion(&self, fixture: &Path, viewport: (u32, u32)) -> io::Result<String> {
        let _ = (fixture, viewport);
        Err(unsupported(self.id(), "motion sampling"))
    }
}

fn unsupported(id: &str, operation: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        format!("instrument `{id}` does not support {operation} (UNAVAILABLE)"),
    )
}

/// Playwright driving headless Chromium: the reference instrument (G43-G52).
pub struct PlaywrightChromium;

impl BrowserInstrument for PlaywrightChromium {
    fn id(&self) -> &str {
        "chromium-playwright"
    }

    fn capabilities(&self) -> InstrumentCapabilities {
        InstrumentCapabilities {
            layout: true,
            interaction: true,
            motion_sampling: true,
            network_isolation: true,
            fresh_context_per_viewport: true,
        }
    }

    fn availability(&self) -> Result<(), String> {
        playwright_module()
            .map(|_| ())
            .ok_or_else(|| "no Playwright module (set ATLAS_PLAYWRIGHT_MODULE)".into())
    }

    fn observe_layout(&self, fixture: &Path, viewports: &[(u32, u32)]) -> io::Result<String> {
        run_instrument(fixture, viewports, "layout")
    }

    fn observe_interactions(&self, fixture: &Path, viewport: (u32, u32)) -> io::Result<String> {
        run_instrument(fixture, &[viewport], "interact")
    }

    fn observe_motion(&self, fixture: &Path, viewport: (u32, u32)) -> io::Result<String> {
        run_instrument(fixture, &[viewport], "motion")
    }
}

/// Which W3C WebDriver server a [`WebDriverInstrument`] speaks to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebDriverKind {
    /// chromedriver + a Chromium binary of the same major version.
    Chromedriver,
    /// Ladybird's `WebDriver` service (`--headless`).
    Ladybird,
}

/// A browser driven through standard W3C WebDriver endpoints only (layout observation).
pub struct WebDriverInstrument {
    pub kind: WebDriverKind,
    pub driver_binary: Option<PathBuf>,
    pub browser_binary: Option<PathBuf>,
    /// The engine's declared layout resolution (`None` = not established: the instrument refuses
    /// to observe rather than guess an uncertainty).
    pub layout_resolution_px: Option<f64>,
}

impl WebDriverInstrument {
    /// chromedriver from `ATLAS_CHROMEDRIVER` (else `chromedriver` on PATH) and Chromium from
    /// `ATLAS_CHROMIUM_BINARY` (else Playwright's installed build).
    pub fn chromedriver() -> Self {
        let driver = std::env::var_os("ATLAS_CHROMEDRIVER")
            .map(PathBuf::from)
            .or_else(|| which("chromedriver"));
        let browser = std::env::var_os("ATLAS_CHROMIUM_BINARY")
            .map(PathBuf::from)
            .or_else(playwright_chromium_binary);
        Self {
            kind: WebDriverKind::Chromedriver,
            driver_binary: driver,
            browser_binary: browser,
            layout_resolution_px: Some(BLINK_LAYOUT_RESOLUTION_PX),
        }
    }

    /// Ladybird's WebDriver from `ATLAS_LADYBIRD_WEBDRIVER`. Layout resolution is established
    /// from source: LibWeb's `CSSPixels` is fixed point with `fractional_bits = 6`
    /// (`Libraries/LibCompositing/PixelUnits.h:69-70` at 1647fd9789ef), i.e. 1/64 px.
    pub fn ladybird() -> Self {
        Self {
            kind: WebDriverKind::Ladybird,
            driver_binary: std::env::var_os("ATLAS_LADYBIRD_WEBDRIVER").map(PathBuf::from),
            browser_binary: None,
            layout_resolution_px: Some(LIBWEB_LAYOUT_RESOLUTION_PX),
        }
    }
}

impl BrowserInstrument for WebDriverInstrument {
    fn id(&self) -> &str {
        match self.kind {
            WebDriverKind::Chromedriver => "chromium-webdriver",
            WebDriverKind::Ladybird => "ladybird",
        }
    }

    fn capabilities(&self) -> InstrumentCapabilities {
        InstrumentCapabilities {
            layout: true,
            interaction: false,
            motion_sampling: false,
            network_isolation: false,
            fresh_context_per_viewport: true,
        }
    }

    fn availability(&self) -> Result<(), String> {
        let driver = self
            .driver_binary
            .as_ref()
            .filter(|p| p.is_file())
            .ok_or_else(|| match self.kind {
                WebDriverKind::Chromedriver => {
                    "no chromedriver (set ATLAS_CHROMEDRIVER)".to_owned()
                }
                WebDriverKind::Ladybird => "no Ladybird WebDriver binary (set \
                    ATLAS_LADYBIRD_WEBDRIVER); Ladybird publishes no prebuilt binaries"
                    .to_owned(),
            })?;
        if self.layout_resolution_px.is_none() {
            return Err("layout resolution not established for this engine".into());
        }
        if self.kind == WebDriverKind::Chromedriver {
            let browser = self
                .browser_binary
                .as_ref()
                .filter(|p| p.is_file())
                .ok_or("no Chromium binary (set ATLAS_CHROMIUM_BINARY)")?;
            let (d, b) = (major_version(driver), major_version(browser));
            if d.is_none() || d != b {
                return Err(format!(
                    "chromedriver major {d:?} does not match Chromium major {b:?}"
                ));
            }
        }
        Ok(())
    }

    fn observe_layout(&self, fixture: &Path, viewports: &[(u32, u32)]) -> io::Result<String> {
        self.availability().map_err(|reason| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("instrument `{}` unavailable: {reason}", self.id()),
            )
        })?;
        let (url, viewport_list, properties) = instrument_inputs(fixture, viewports)?;
        let script = format!("{MEASURE_LAYOUT_SCRIPT}\n{WEBDRIVER_SCRIPT}");
        let kind = match self.kind {
            WebDriverKind::Chromedriver => "chromedriver",
            WebDriverKind::Ladybird => "ladybird",
        };
        let mut command = Command::new("node");
        command
            .arg("-e")
            .arg(script)
            .env("ATLAS_WEBDRIVER_KIND", kind)
            .env(
                "ATLAS_WEBDRIVER_BINARY",
                self.driver_binary.as_ref().expect("available"),
            )
            .env("ATLAS_OBSERVE_URL", &url)
            .env("ATLAS_OBSERVE_VIEWPORTS", viewport_list)
            .env("ATLAS_OBSERVE_PROPERTIES", properties)
            .env(
                "ATLAS_OBSERVE_ELEMENT_LIMIT",
                OBSERVATION_ELEMENT_LIMIT.to_string(),
            )
            .env(
                "ATLAS_LAYOUT_RESOLUTION_PX",
                self.layout_resolution_px.expect("available").to_string(),
            );
        if let Some(browser) = &self.browser_binary {
            command.env("ATLAS_BROWSER_BINARY", browser);
        }
        run_node(command)
    }
}

/// Every instrument this build knows, in registry order.
pub fn instruments() -> Vec<Box<dyn BrowserInstrument>> {
    vec![
        Box::new(PlaywrightChromium),
        Box::new(WebDriverInstrument::chromedriver()),
        Box::new(WebDriverInstrument::ladybird()),
    ]
}

/// The instrument with registry id `id`.
pub fn instrument(id: &str) -> Option<Box<dyn BrowserInstrument>> {
    instruments().into_iter().find(|i| i.id() == id)
}

fn which(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(name))
            .find(|candidate| candidate.is_file())
    })
}

/// Playwright's installed Chromium (`$PLAYWRIGHT_BROWSERS_PATH/chromium-*/chrome-linux/chrome`).
fn playwright_chromium_binary() -> Option<PathBuf> {
    let root = PathBuf::from(std::env::var_os("PLAYWRIGHT_BROWSERS_PATH")?);
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("chromium-"))
        })
        .map(|path| path.join("chrome-linux/chrome"))
        .filter(|path| path.is_file())
        .collect();
    candidates.sort();
    candidates.pop()
}

/// The leading integer of the first dotted version in `binary --version`.
fn major_version(binary: &Path) -> Option<u32> {
    let output = Command::new(binary).arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.split_whitespace()
        .find(|word| word.contains('.') && word.chars().next().is_some_and(|c| c.is_ascii_digit()))?
        .split('.')
        .next()?
        .parse()
        .ok()
}

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

/// Hovers every interactive candidate of `fixture` and deterministically samples each resulting
/// animation's property curve (ADR 0016).
pub fn motion_fixture_raw(fixture: &Path, viewport: (u32, u32)) -> io::Result<String> {
    run_instrument(fixture, &[viewport], "motion")
}

/// Validates the subject and viewports and renders the harness inputs (subject URL, viewport
/// list, observed property list).
fn instrument_inputs(
    fixture: &Path,
    viewports: &[(u32, u32)],
) -> io::Result<(String, String, String)> {
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
    Ok((url, viewport_list, properties))
}

fn run_instrument(fixture: &Path, viewports: &[(u32, u32)], mode: &str) -> io::Result<String> {
    let (url, viewport_list, properties) = instrument_inputs(fixture, viewports)?;
    let module = playwright_module().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "browser instrument unavailable: no Playwright module (set ATLAS_PLAYWRIGHT_MODULE)",
        )
    })?;
    let mut command = Command::new("node");
    command
        .arg("-e")
        .arg(format!("{MEASURE_LAYOUT_SCRIPT}\n{OBSERVATION_SCRIPT}"))
        .env("ATLAS_PLAYWRIGHT_MODULE", &module)
        .env("ATLAS_OBSERVE_URL", &url)
        .env("ATLAS_OBSERVE_MODE", mode)
        .env("ATLAS_OBSERVE_VIEWPORTS", viewport_list)
        .env("ATLAS_OBSERVE_PROPERTIES", properties)
        .env(
            "ATLAS_OBSERVE_ELEMENT_LIMIT",
            OBSERVATION_ELEMENT_LIMIT.to_string(),
        );
    run_node(command)
}

fn run_node(mut command: Command) -> io::Result<String> {
    let output = command.output()?;
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
