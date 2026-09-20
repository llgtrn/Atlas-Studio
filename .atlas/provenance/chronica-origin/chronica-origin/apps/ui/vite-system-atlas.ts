import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Plugin, ViteDevServer } from 'vite';

const WATCH_PREFIXES = ['.github/', '.chronica/', 'docs/', 'core/', 'runtime/', 'adapter/', 'organism/', 'apps/', 'graph/', 'bindings/', 'deploy/', 'tools/', 'scripts/'];
const WATCH_ROOT_FILES = new Set([
  'README.md', 'AGENTS.md', 'Dockerfile', '.env.example', '.dockerignore', '.gitignore', '.gitguardian.yaml',
  'pnpm-workspace.yaml', 'pnpm-lock.yaml', 'package.json', 'Cargo.toml', 'Cargo.lock', 'deny.toml',
]);
const WATCH_EXTENSIONS = new Set([
  '.md', '.rs', '.ts', '.tsx', '.js', '.mjs', '.cjs', '.sql', '.proto', '.toml', '.json', '.yaml', '.yml',
  '.html', '.css', '.svg', '.png', '.ico', '.webmanifest', '.sh', '.py', '.go', '.c', '.cc', '.cpp', '.h', '.hpp',
]);
const UI_DIR = path.dirname(fileURLToPath(import.meta.url));

function slash(value: string) {
  return value.split(path.sep).join('/');
}

function isAtlasRelevant(repoRoot: string, file: string) {
  const rel = slash(path.relative(repoRoot, file));
  if (!rel || rel.startsWith('..')) return false;
  if (rel === 'apps/ui/public/docs-atlas.json') return false;
  if (WATCH_ROOT_FILES.has(rel)) return true;
  if (!WATCH_PREFIXES.some((prefix) => rel.startsWith(prefix))) return false;
  return WATCH_EXTENSIONS.has(path.extname(rel)) || rel.endsWith('Cargo.toml') || rel.endsWith('package.json');
}

export function systemAtlasRealityPlugin(): Plugin {
  const repoRoot = path.resolve(UI_DIR, '../..');
  const buildScript = path.resolve(repoRoot, 'tools/docs-atlas/build.mjs');
  const output = 'apps/ui/public/docs-atlas.json';
  let timer: NodeJS.Timeout | null = null;
  let building = false;
  let pending = false;

  const runBuild = (server: ViteDevServer) => {
    if (building) {
      pending = true;
      return;
    }
    building = true;
    const child = spawn(process.execPath, [buildScript, '--output', output], {
      cwd: repoRoot,
      stdio: 'inherit',
    });
    child.on('close', (code) => {
      building = false;
      if (code === 0) server.ws.send({ type: 'full-reload', path: '*' });
      if (pending) {
        pending = false;
        runBuild(server);
      }
    });
  };

  const schedule = (server: ViteDevServer, file: string) => {
    if (!isAtlasRelevant(repoRoot, file)) return;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => runBuild(server), 450);
  };

  return {
    name: 'chronica-system-atlas-reality-mirror',
    configureServer(server) {
      for (const prefix of WATCH_PREFIXES) server.watcher.add(path.resolve(repoRoot, prefix));
      for (const file of WATCH_ROOT_FILES) server.watcher.add(path.resolve(repoRoot, file));
      server.watcher.on('add', (file) => schedule(server, file));
      server.watcher.on('change', (file) => schedule(server, file));
      server.watcher.on('unlink', (file) => schedule(server, file));
    },
  };
}
