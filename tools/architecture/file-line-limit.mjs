import { readdirSync, readFileSync } from "node:fs";
import { extname, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const DEFAULT_LIMIT = 600;
const DEFAULT_EXTENSIONS = [".rs", ".mjs", ".js", ".ts", ".tsx", ".md", ".toml", ".json"];
const DEFAULT_IGNORED_DIRS = new Set([
  ".git",
  ".codex-remote-attachments",
  "node_modules",
  "target",
  "Temporary",
  ".archive",
  "_machine",
]);

function normalizePath(path) {
  return path.split(sep).join("/");
}

function countLines(path) {
  const text = readFileSync(path, "utf8");
  if (text.length === 0) return 0;
  return text.split(/\r?\n/).length;
}

function walk(root, options, out) {
  const entries = readdirSync(root, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name));
  for (const entry of entries) {
    if (entry.isDirectory()) {
      if (!options.ignoredDirs.has(entry.name)) {
        walk(resolve(root, entry.name), options, out);
      }
      continue;
    }

    if (!entry.isFile()) continue;
    const path = resolve(root, entry.name);
    if (!options.extensions.has(extname(entry.name))) continue;
    // Extracted test-module files are EXEMPT: the 600-line gate forces #[path] test extraction
    // from oversized capability modules, and the extracted test body is a verbatim move whose
    // size is set by the donor's edge-case count — splitting it further would change test-mod
    // paths and break the verbatim guarantee. The PARENT (prod) file stays fully gated.
    if (entry.name.endsWith("_tests.rs")) continue;
    // UI test/spec files are EXEMPT (mirrors _tests.rs): they are verbatim render/interaction
    // nets whose size tracks the surface under test, not production God-Files.
    if (/\.(test|spec)\.[jt]sx?$/.test(entry.name)) continue;
    const lines = countLines(path);
    if (lines > options.limit) {
      out.push({
        absolutePath: path,
        relativePath: normalizePath(relative(options.root, path)),
        lines,
      });
    }
  }
}

export function scanFileLineLimits(root, config = {}) {
  const options = {
    root: resolve(root),
    limit: config.limit ?? DEFAULT_LIMIT,
    extensions: new Set(config.extensions ?? DEFAULT_EXTENSIONS),
    ignoredDirs: new Set([...(config.ignoredDirs ?? DEFAULT_IGNORED_DIRS)]),
  };
  const violations = [];
  const includePaths = config.includePaths?.length ? config.includePaths : ["."];
  for (const includePath of includePaths) {
    walk(resolve(options.root, includePath), options, violations);
  }
  violations.sort((a, b) => b.lines - a.lines || a.relativePath.localeCompare(b.relativePath));
  return { limit: options.limit, violations };
}

function parseArgs(argv) {
  const config = { fail: false, includePaths: [], limit: DEFAULT_LIMIT, root: process.cwd(), top: 50 };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--fail") config.fail = true;
    else if (arg === "--limit") config.limit = Number(argv[++i]);
    else if (arg === "--path") config.includePaths.push(argv[++i]);
    else if (arg === "--top") config.top = Number(argv[++i]);
    else if (arg === "--root") config.root = argv[++i];
    else throw new Error(`unknown argument: ${arg}`);
  }
  if (!Number.isInteger(config.limit) || config.limit < 1) {
    throw new Error("--limit must be a positive integer");
  }
  if (!Number.isInteger(config.top) || config.top < 1) {
    throw new Error("--top must be a positive integer");
  }
  return config;
}

function main() {
  const config = parseArgs(process.argv.slice(2));
  const result = scanFileLineLimits(config.root, {
    includePaths: config.includePaths,
    limit: config.limit,
  });
  console.log(`line_limit=${result.limit}`);
  console.log(`violations=${result.violations.length}`);
  for (const violation of result.violations.slice(0, config.top)) {
    console.log(`${violation.lines}\t${violation.relativePath}`);
  }
  if (config.fail && result.violations.length > 0) {
    process.exitCode = 1;
  }
}

const currentFile = fileURLToPath(import.meta.url);
if (process.argv[1] && resolve(process.argv[1]) === currentFile) {
  main();
}
