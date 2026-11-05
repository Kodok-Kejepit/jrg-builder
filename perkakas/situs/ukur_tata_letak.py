#!/usr/bin/env python3
"""Ukur tata letak situs di chromium headless: kepala tidak menimpa isi, tidak ada gulir mendatar
(juga di lebar HP), dan halaman benar-benar berisi. Pemeriksa statis tidak bisa melihat tumpang-tindih."""
import json, re, shutil, subprocess, sys
from pathlib import Path

DOCS = Path(__file__).resolve().parent.parent.parent / "docs"
HALAMAN = ["index.html", "web_dokumentasi/index.html", "web_dokumentasi/tipe.html", "web_dokumentasi/jrgc.html",
           "matematika_x_jrg/index.html", "matematika_x_jrg/ch1.html", "matematika_x_jrg/jrg12.html"]
LEBAR = [1280, 390]

SUNTIKAN = """
<script>
setTimeout(function(){
  var k = document.querySelector('.kepala'), s = document.querySelector('.sisi'), i = document.querySelector('#isi');
  var r = {
    kepala: k ? Math.round(k.getBoundingClientRect().height) : -1,
    sisiAtas: s && getComputedStyle(s).display !== 'none' ? Math.round(s.getBoundingClientRect().top) : null,
    isiAtas: i ? Math.round(i.getBoundingClientRect().top) : null,
    gulirX: document.documentElement.scrollWidth > window.innerWidth + 1 ? document.documentElement.scrollWidth : 0,
    panjangTeks: document.body.innerText.trim().length,
    tombolKode: document.querySelectorAll('figure.kode .alat button').length,
    kode: document.querySelectorAll('figure.kode').length
  };
  var d = document.createElement('div'); d.id = 'HASIL-UKUR'; d.textContent = JSON.stringify(r);
  document.body.appendChild(d);
}, 800);
</script>
"""


def krom():
    return shutil.which("chromium") or shutil.which("chromium-browser") or shutil.which("google-chrome")


def ukur(path: Path, lebar: int):
    salinan = path.with_name(f"__ukur__{path.name}")
    try:
        salinan.write_text(path.read_text(encoding="utf-8").replace("</body>", SUNTIKAN + "</body>"), encoding="utf-8")
        r = subprocess.run([krom(), "--headless", "--disable-gpu", "--no-sandbox", "--hide-scrollbars",
                            "--password-store=basic", f"--window-size={lebar},900", "--virtual-time-budget=4000",
                            "--dump-dom", salinan.as_uri()], capture_output=True, text=True, timeout=120)
    finally:
        salinan.unlink(missing_ok=True)
    m = re.search(r'id="HASIL-UKUR">([^<]*)<', r.stdout)
    return json.loads(m.group(1)) if m else None


def main():
    if not krom():
        print("chromium tidak ada: pengukuran tata letak DILEWATI (bukan lulus)")
        return 0
    salah = []
    for h in HALAMAN:
        p = DOCS / h
        if not p.exists():
            salah.append(f"{h}: berkas tidak ada")
            continue
        for lebar in LEBAR:
            r = ukur(p, lebar)
            nama = f"{h} @{lebar}"
            if r is None:
                salah.append(f"{nama}: pengukuran gagal (skrip tidak jalan?)")
                continue
            print(f"  {nama:38} kepala {r['kepala']}px  sisi@{r['sisiAtas']}  isi@{r['isiAtas']}  "
                  f"teks {r['panjangTeks']}  kode {r['kode']}")
            if r["kepala"] < 40:
                salah.append(f"{nama}: kepala {r['kepala']}px, gaya tidak termuat?")
            for label, atas in (("sisi", r["sisiAtas"]), ("isi", r["isiAtas"])):
                if atas is not None and atas < r["kepala"] - 1:
                    salah.append(f"{nama}: {label} mulai di {atas}px, tertimpa kepala {r['kepala']}px")
            if r["gulirX"]:
                salah.append(f"{nama}: gulir mendatar (lebar halaman {r['gulirX']}px)")
            if r["panjangTeks"] < 400:
                salah.append(f"{nama}: isinya cuma {r['panjangTeks']} karakter")
            if r["kode"] and r["tombolKode"] < r["kode"]:
                salah.append(f"{nama}: {r['kode']} blok kode tapi cuma {r['tombolKode']} tombol; situs.js tidak jalan?")
    if salah:
        print(f"\n{len(salah)} MASALAH TATA LETAK:")
        for s in salah:
            print("  -", s)
        return 1
    print("TATA LETAK BERSIH")
    return 0


if __name__ == "__main__":
    sys.exit(main())
