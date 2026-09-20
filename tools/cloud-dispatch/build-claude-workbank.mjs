#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { createReadStream, createWriteStream } from 'node:fs';
import { mkdir, readdir, readFile, stat, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import readline from 'node:readline';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(HERE, '..', '..');

const args = new Map();
for (let i = 2; i < process.argv.length; i++) {
  const arg = process.argv[i];
  if (arg.startsWith('--')) {
    const key = arg.slice(2);
    const next = process.argv[i + 1];
    if (!next || next.startsWith('--')) args.set(key, 'true');
    else {
      args.set(key, next);
      i++;
    }
  }
}

const source = path.resolve(args.get('source') || process.env.CLAUDE_CHRONICA_CORPUS || '');
const outDir = path.resolve(REPO, args.get('out') || 'tools/cloud-dispatch/claude-workbank');
const maxRecords = Number(args.get('max-records') || 3000);
const maxChars = Number(args.get('max-chars') || 6000);
const includeSubagents = args.get('include-subagents') === 'true';

if (!source) fail('Missing --source or CLAUDE_CHRONICA_CORPUS.');
const displaySource = redactLocalPath(source);

const secretPatterns = [
  ['openai_key', /\bsk-[A-Za-z0-9_-]{20,}\b/g],
  ['anthropic_key', /\bsk-ant-[A-Za-z0-9_-]{20,}\b/g],
  ['github_token', /\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9_]{20,}\b/g],
  ['github_pat', /\bgithub_pat_[A-Za-z0-9_]{20,}\b/g],
  ['aws_access_key', /\bAKIA[0-9A-Z]{16}\b/g],
  ['telegram_bot_token', /\b\d{8,12}:[A-Za-z0-9_-]{30,}\b/g],
  ['jwt', /\beyJ[A-Za-z0-9_-]{20,}\.[A-Za-z0-9_-]{20,}\.[A-Za-z0-9_-]{20,}\b/g],
  ['generic_assignment_secret', /\b(?:api[_-]?key|secret|token|password|passwd|pwd)\s*[:=]\s*["']?[^"'\s]{12,}/gi],
];

const summary = {
  generated_at: new Date().toISOString(),
  source_path: displaySource,
  include_subagents: includeSubagents,
  max_records: maxRecords,
  max_chars: maxChars,
  files_seen: 0,
  bytes_seen: 0,
  manifest_entries: 0,
  records_written: 0,
  records_skipped_after_limit: 0,
  parse_errors: 0,
  secret_flags: Object.fromEntries(secretPatterns.map(([name]) => [name, 0])),
  extension_counts: {},
};

await mkdir(outDir, { recursive: true });
await mkdir(path.join(outDir, 'raw'), { recursive: true });

const files = await walk(source);
files.sort((a, b) => a.localeCompare(b));

const manifest = createWriteStream(path.join(outDir, 'manifest.jsonl'), { encoding: 'utf8' });
const records = createWriteStream(path.join(outDir, 'records.jsonl'), { encoding: 'utf8' });

for (const abs of files) {
  const rel = path.relative(source, abs).split(path.sep).join('/');
  if (!includeSubagents && rel.includes('/subagents/')) continue;
  if (rel.startsWith('memory/')) continue;

  const st = await stat(abs);
  const ext = path.extname(abs).toLowerCase() || '(none)';
  summary.files_seen++;
  summary.bytes_seen += st.size;
  summary.extension_counts[ext] = (summary.extension_counts[ext] || 0) + 1;

  const sha256 = await hashFile(abs);
  manifest.write(JSON.stringify({
    rel,
    bytes: st.size,
    ext,
    sha256,
    mtime: st.mtime.toISOString(),
  }) + '\n');
  summary.manifest_entries++;

  if (ext === '.jsonl') {
    await extractJsonlRecords(abs, rel, records);
  } else if (['.md', '.txt'].includes(ext) && summary.records_written < maxRecords) {
    const text = await readFile(abs, 'utf8').catch(() => '');
    if (text.trim().length > 80) {
      writeRecord(records, {
        source_file: rel,
        source_kind: ext.slice(1),
        text,
      });
    }
  }
}

await end(manifest);
await end(records);

await writeFile(path.join(outDir, 'source-summary.json'), JSON.stringify({
  ...summary,
  bytes_seen_mb: Number((summary.bytes_seen / 1024 / 1024).toFixed(2)),
}, null, 2) + '\n');

await writeFile(path.join(outDir, 'README.md'), `# Claude Workbank Snapshot

This directory is a cloud-safe snapshot derived from the local Claude corpus for Chronica.

- Raw source path: \`${displaySource.replace(/\\/g, '/')}\`
- Files indexed: ${summary.files_seen}
- Bytes indexed: ${summary.bytes_seen} (${(summary.bytes_seen / 1024 / 1024).toFixed(2)} MB)
- Records exported: ${summary.records_written}
- Generated at: ${summary.generated_at}

The raw corpus is intentionally not committed here. Use \`manifest.jsonl\` for file identity and \`records.jsonl\` as bounded, redacted candidate work records for Codex Cloud.

Cloud agents must treat these records as leads only. Repo code, tests, docs, \`docs/capabilities.db\`, \`docs/architecture.db\`, and verifier output remain the truth sources.
`);

console.log(JSON.stringify(summary, null, 2));

function fail(message) {
  console.error(`build-claude-workbank: ${message}`);
  process.exit(2);
}

async function walk(dir) {
  const out = [];
  for (const ent of await readdir(dir, { withFileTypes: true })) {
    const abs = path.join(dir, ent.name);
    if (ent.isDirectory()) out.push(...await walk(abs));
    else if (ent.isFile()) out.push(abs);
  }
  return out;
}

async function hashFile(abs) {
  const hash = createHash('sha256');
  await new Promise((resolve, reject) => {
    const stream = createReadStream(abs);
    stream.on('data', (chunk) => hash.update(chunk));
    stream.on('error', reject);
    stream.on('end', resolve);
  });
  return hash.digest('hex');
}

async function extractJsonlRecords(abs, rel, out) {
  const rl = readline.createInterface({
    input: createReadStream(abs, { encoding: 'utf8' }),
    crlfDelay: Infinity,
  });
  let lineNo = 0;
  for await (const line of rl) {
    lineNo++;
    if (summary.records_written >= maxRecords) {
      summary.records_skipped_after_limit++;
      continue;
    }
    if (!line.trim()) continue;
    let obj;
    try {
      obj = JSON.parse(line);
    } catch {
      summary.parse_errors++;
      continue;
    }
    const text = extractText(obj);
    if (!text || text.length < 80) continue;
    const role = obj.role || obj.message?.role || obj.type || obj.operation || null;
    writeRecord(out, {
      source_file: rel,
      source_line: lineNo,
      session_id: obj.sessionId || obj.session_id || obj.session?.id || null,
      timestamp: obj.timestamp || obj.created_at || obj.createdAt || null,
      source_kind: 'jsonl',
      role,
      text,
    });
  }
}

function extractText(obj) {
  const candidates = [
    obj.content,
    obj.prompt,
    obj.query,
    obj.text,
    obj.message?.content,
    obj.request?.prompt,
    obj.request?.content,
    obj.response?.content,
  ];
  for (const value of candidates) {
    const text = stringifyContent(value);
    if (text && text.length > 0) return text;
  }
  return '';
}

function stringifyContent(value) {
  if (typeof value === 'string') return value;
  if (Array.isArray(value)) {
    return value.map((item) => stringifyContent(item?.text ?? item?.content ?? item)).filter(Boolean).join('\n');
  }
  if (value && typeof value === 'object') {
    if (typeof value.text === 'string') return value.text;
    if (typeof value.content === 'string') return value.content;
    return JSON.stringify(value);
  }
  return '';
}

function writeRecord(out, record) {
  const original = record.text;
  const { redacted, flags } = redact(original);
  const truncated = redacted.length > maxChars;
  const text = truncated ? `${redacted.slice(0, maxChars)}\n[TRUNCATED ${redacted.length - maxChars} chars]` : redacted;
  for (const flag of flags) summary.secret_flags[flag] = (summary.secret_flags[flag] || 0) + 1;
  out.write(JSON.stringify({
    ...record,
    text,
    original_chars: original.length,
    exported_chars: text.length,
    truncated,
    redaction_flags: flags,
  }) + '\n');
  summary.records_written++;
}

function redact(text) {
  let redacted = redactLocalPath(text);
  const flags = new Set();
  for (const [name, pattern] of secretPatterns) {
    redacted = redacted.replace(pattern, () => {
      flags.add(name);
      return `[REDACTED_${name.toUpperCase()}]`;
    });
  }
  return { redacted, flags: [...flags].sort() };
}

function redactLocalPath(text) {
  return text.replaceAll('C:\\Users\\trngh', '%USERPROFILE%');
}

function end(stream) {
  return new Promise((resolve, reject) => {
    stream.end(resolve);
    stream.on('error', reject);
  });
}
