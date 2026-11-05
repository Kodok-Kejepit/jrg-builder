// perkakas/situs/praktik_peramban.js — buka `url` di chromium headless lewat
// DevTools Protocol dan tunggu #HASIL-PRAKTIK dengan WAKTU SUNGGUHAN: waktu
// virtual `--virtual-time-budget` mempercepat batas waktu pekerja jadi seketika.
'use strict';
const { spawn } = require('child_process');
const fs = require('fs'), os = require('os'), path = require('path');
const [krom, url] = process.argv.slice(2);
const profil = fs.mkdtempSync(path.join(os.tmpdir(), 'jrg-praktik-'));
// `--password-store=basic`: tanpa itu chromium menunggu keyring desktop dan bisa menggantung.
const proses = spawn(krom, ['--headless', '--disable-gpu', '--no-sandbox', '--password-store=basic', '--remote-debugging-port=0',
  '--user-data-dir=' + profil, 'about:blank'], { stdio: ['ignore', 'ignore', 'pipe'] });
const akhiri = (kode, pesan) => {
  if (pesan) process.stdout.write(pesan);
  proses.kill('SIGKILL');
  fs.rmSync(profil, { recursive: true, force: true });
  process.exit(kode);
};
const batas = setTimeout(() => akhiri(2, 'BATAS WAKTU'), 120000);
let buf = '', tersambung = false;
proses.stderr.on('data', (d) => {
  buf += d;
  const m = buf.match(/DevTools listening on (ws:\/\/\S+)/);
  if (!m || tersambung) return;
  tersambung = true;
  const ws = new WebSocket(m[1]);
  let id = 0;
  const tunggu = new Map();
  const kirim = (method, params, sessionId) => new Promise((ok) => {
    const i = ++id;
    tunggu.set(i, ok);
    ws.send(JSON.stringify(Object.assign({ id: i, method, params: params || {} }, sessionId ? { sessionId } : {})));
  });
  ws.onmessage = (e) => {
    const d = JSON.parse(e.data);
    if (d.id && tunggu.has(d.id)) { tunggu.get(d.id)(d); tunggu.delete(d.id); }
  };
  ws.onopen = async () => {
    const t = await kirim('Target.createTarget', { url: 'about:blank' });
    const s = await kirim('Target.attachToTarget', { targetId: t.result.targetId, flatten: true });
    const sid = s.result.sessionId;
    await kirim('Page.navigate', { url }, sid);
    for (;;) {
      await new Promise((r) => setTimeout(r, 250));
      const r = await kirim('Runtime.evaluate', {
        expression: "(document.getElementById('HASIL-PRAKTIK')||{}).textContent || null", returnByValue: true }, sid);
      const v = r.result && r.result.result && r.result.result.value;
      if (v) { clearTimeout(batas); ws.close(); akhiri(0, v); }
    }
  };
});
