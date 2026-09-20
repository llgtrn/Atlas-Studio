#!/usr/bin/env node
// Telegram notifier for the Chronica Brain/Hands loop.
// Token + chat id are read from ~/.chronica-telegram.env (OUTSIDE the repo — never committed).
// Usage:
//   node tools/agent/tg-notify.mjs <message-file>     # send the UTF-8 contents of a file
//   node tools/agent/tg-notify.mjs --text "inline"    # send an inline string
import fs from 'node:fs';
import os from 'node:os';
import https from 'node:https';
import path from 'node:path';

const cfgPath = path.join(os.homedir(), '.chronica-telegram.env');
let token = '', chat = '';
try {
  for (const line of fs.readFileSync(cfgPath, 'utf8').split(/\r?\n/)) {
    const m = line.match(/^(TG_TOKEN|TG_CHAT)=(.*)$/);
    if (m) { if (m[1] === 'TG_TOKEN') token = m[2].trim(); else chat = m[2].trim(); }
  }
} catch (e) { console.error('no telegram config at ' + cfgPath); process.exit(2); }
if (!token || !chat) { console.error('TG_TOKEN/TG_CHAT missing in ' + cfgPath); process.exit(2); }

const args = process.argv.slice(2);
let text = '';
if (args[0] === '--text') text = args.slice(1).join(' ');
else if (args[0]) text = fs.readFileSync(args[0], 'utf8');
else { console.error('usage: tg-notify.mjs <file> | --text "..."'); process.exit(2); }
if (!text.trim()) { console.error('empty message'); process.exit(2); }

const body = JSON.stringify({ chat_id: chat, text, disable_web_page_preview: true });
const req = https.request({
  hostname: 'api.telegram.org',
  path: `/bot${token}/sendMessage`,
  method: 'POST',
  headers: { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(body) },
  timeout: 20000,
}, (res) => {
  let s = '';
  res.on('data', (d) => (s += d));
  res.on('end', () => {
    try { const j = JSON.parse(s); console.log(j.ok ? ('SENT ok message_id=' + j.result.message_id) : ('FAIL: ' + JSON.stringify(j).slice(0, 300))); process.exit(j.ok ? 0 : 1); }
    catch { console.log('parse: ' + s.slice(0, 200)); process.exit(1); }
  });
});
req.on('error', (e) => { console.error('net error: ' + e.message); process.exit(1); });
req.on('timeout', () => { req.destroy(); console.error('timeout'); process.exit(1); });
req.write(body);
req.end();
