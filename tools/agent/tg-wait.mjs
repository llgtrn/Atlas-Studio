#!/usr/bin/env node
// Long-poll Telegram until the operator sends a message, then exit (wakes the loop near-real-time).
// Exits 0 printing `MSG\t<text>` on a message, or `(timeout)` after the max window (re-arm cadence).
// Token/chat from ~/.chronica-telegram.env; offset persisted in ~/.chronica-telegram.offset.
import fs from 'node:fs';
import os from 'node:os';
import https from 'node:https';
import path from 'node:path';

const home = os.homedir();
const cfgPath = path.join(home, '.chronica-telegram.env');
const offPath = path.join(home, '.chronica-telegram.offset');
let token = '', chat = '';
for (const line of fs.readFileSync(cfgPath, 'utf8').split(/\r?\n/)) {
  const m = line.match(/^(TG_TOKEN|TG_CHAT)=(.*)$/);
  if (m) { if (m[1] === 'TG_TOKEN') token = m[2].trim(); else chat = m[2].trim(); }
}
let offset = 0;
try { offset = parseInt(fs.readFileSync(offPath, 'utf8').trim(), 10) || 0; } catch {}

const MAX_MS = parseInt(process.argv[2] || '240', 10) * 1000; // total window before a re-arm tick
const POLL = 45; // per-call long-poll seconds
const deadline = Date.now() + MAX_MS;

function once() {
  return new Promise((resolve) => {
    const left = Math.max(1, Math.min(POLL, Math.ceil((deadline - Date.now()) / 1000)));
    https.get(`https://api.telegram.org/bot${token}/getUpdates?timeout=${left}&offset=${offset}`,
      { timeout: (left + 10) * 1000 }, (res) => {
        let s = ''; res.on('data', (d) => (s += d));
        res.on('end', () => { try { resolve(JSON.parse(s)); } catch { resolve({ ok: false }); } });
      }).on('error', () => resolve({ ok: false })).on('timeout', function () { this.destroy(); resolve({ ok: false }); });
  });
}

(async () => {
  while (Date.now() < deadline) {
    const j = await once();
    if (j && j.ok && j.result && j.result.length) {
      let maxId = offset - 1; const msgs = [];
      for (const u of j.result) {
        if (u.update_id > maxId) maxId = u.update_id;
        const m = u.message || u.edited_message;
        if (m && String(m.chat?.id) === String(chat) && m.text) msgs.push(m.text);
      }
      fs.writeFileSync(offPath, String(maxId + 1));
      if (msgs.length) { for (const t of msgs) console.log('MSG\t' + t.replace(/\n/g, ' ⏎ ')); process.exit(0); }
    }
  }
  console.log('(timeout)');
  process.exit(0);
})();
