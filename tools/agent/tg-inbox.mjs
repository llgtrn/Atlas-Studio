#!/usr/bin/env node
// Poll Telegram for NEW messages from the operator (two-way control of the loop).
// Token/chat from ~/.chronica-telegram.env; offset persisted in ~/.chronica-telegram.offset.
// Prints each new message from the configured chat as: MSG\t<text>. Prints nothing if none.
import fs from 'node:fs';
import os from 'node:os';
import https from 'node:https';
import path from 'node:path';

const home = os.homedir();
const cfgPath = path.join(home, '.chronica-telegram.env');
const offPath = path.join(home, '.chronica-telegram.offset');
let token = '', chat = '';
try {
  for (const line of fs.readFileSync(cfgPath, 'utf8').split(/\r?\n/)) {
    const m = line.match(/^(TG_TOKEN|TG_CHAT)=(.*)$/);
    if (m) { if (m[1] === 'TG_TOKEN') token = m[2].trim(); else chat = m[2].trim(); }
  }
} catch { console.error('no telegram config'); process.exit(2); }
let offset = 0;
try { offset = parseInt(fs.readFileSync(offPath, 'utf8').trim(), 10) || 0; } catch {}

https.get(`https://api.telegram.org/bot${token}/getUpdates?timeout=0&offset=${offset}`, { timeout: 20000 }, (res) => {
  let s = '';
  res.on('data', (d) => (s += d));
  res.on('end', () => {
    let j; try { j = JSON.parse(s); } catch { console.error('parse fail'); process.exit(1); }
    if (!j.ok) { console.error('FAIL: ' + JSON.stringify(j).slice(0, 200)); process.exit(1); }
    let maxId = offset - 1, n = 0;
    for (const u of j.result) {
      if (u.update_id > maxId) maxId = u.update_id;
      const msg = u.message || u.edited_message;
      if (msg && String(msg.chat?.id) === String(chat) && msg.text) {
        console.log('MSG\t' + msg.text.replace(/\n/g, ' ⏎ '));
        n++;
      }
    }
    if (j.result.length) fs.writeFileSync(offPath, String(maxId + 1));
    if (!n) console.log('(no new operator messages)');
    process.exit(0);
  });
}).on('error', (e) => { console.error('net: ' + e.message); process.exit(1); });
