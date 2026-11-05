#!/usr/bin/env python3
"""JRG IDE — varian/tambal_manifes.py: ubah AndroidManifest.xml biner (AXML) APK buatan `jrgm apk`.

Pakai: tambal_manifes.py <masuk.apk> <keluar-belum-bertanda.apk> <targetSdk> [--latar] [<jalur-di-apk>=<berkas> ...]
- targetSdk: versi Terminal butuh 28 supaya program yang diunduh boleh dijalankan.
- --latar: daftarkan layanan latar ide/varian/latar/Latar.java (classes2.dex ikut lewat jalur=berkas), izinnya
  (notifikasi, wake lock, tulis memori HP), dan aktivitas yang tidak dibuat ulang saat layar/huruf berubah.
- jalur=berkas: berkas tambahan tanpa kompresi (proot di lib/<abi>/, classes2.dex).
Manifesnya diurai utuh lalu ditulis ulang; atribut tetap urut menurut id sumber dayanya, seperti buatan aapt2.
"""
import struct
import sys
import zipfile

NS_ANDROID = 'http://schemas.android.com/apk/res/android'
ATTR = {'name': 0x01010003, 'exported': 0x01010010, 'authorities': 0x01010018,
        'configChanges': 0x0101001f, 'targetSdkVersion': 0x01010270}
T_STRING, T_HEX, T_DEC, T_BOOL = 0x03, 0x11, 0x10, 0x12
# mcc mnc locale touchscreen keyboard keyboardHidden navigation orientation screenLayout uiMode screenSize
# smallestScreenSize density layoutDirection colorMode fontScale: aktivitas (dan pelayannya) tidak dibuat ulang.
CONFIG_SEMUA = 0x1 | 0x2 | 0x4 | 0x8 | 0x10 | 0x20 | 0x40 | 0x80 | 0x100 | 0x200 | 0x400 | 0x800 | 0x1000 | 0x2000 | 0x4000 | 0x40000000
IZIN_LATAR = ['android.permission.FOREGROUND_SERVICE', 'android.permission.WAKE_LOCK', 'android.permission.POST_NOTIFICATIONS',
              'android.permission.READ_EXTERNAL_STORAGE', 'android.permission.WRITE_EXTERNAL_STORAGE']
TANPA = 0xFFFFFFFF


class Galat(SystemExit):
    pass


# ---------- urai ----------

def baca_kolam(data, i):
    _, kp, _, jumlah, gaya, bendera, mulai, _ = struct.unpack_from('<HHIIIIII', data, i)
    if gaya:
        raise Galat('kolam string manifes punya gaya; tidak dikenal')
    utf8 = bool(bendera & 0x100)
    hasil = []
    for k in range(jumlah):
        o = i + mulai + struct.unpack_from('<I', data, i + kp + 4 * k)[0]
        if utf8:
            o += 2 if data[o] & 0x80 else 1
            n = data[o]
            if n & 0x80:
                n = ((n & 0x7f) << 8) | data[o + 1]
                o += 1
            hasil.append(data[o + 1:o + 1 + n].decode('utf-8'))
        else:
            n = struct.unpack_from('<H', data, o)[0]
            if n & 0x8000:
                n = ((n & 0x7fff) << 16) | struct.unpack_from('<H', data, o + 2)[0]
                o += 2
            hasil.append(data[o + 2:o + 2 + 2 * n].decode('utf-16-le'))
    return hasil, utf8


def urai(data):
    jenis, kepala, ukuran = struct.unpack_from('<HHI', data, 0)
    if jenis != 0x0003:
        raise Galat('AndroidManifest.xml bukan XML biner')
    s, peta, simpul, utf8 = [], [], [], False
    i = kepala
    while i < ukuran:
        j, kp, uk = struct.unpack_from('<HHI', data, i)
        st = lambda x: None if x == TANPA else s[x]
        if j == 0x0001:
            s, utf8 = baca_kolam(data, i)
        elif j == 0x0180:
            peta = list(struct.unpack_from('<%dI' % ((uk - kp) // 4), data, i + kp))
        elif j in (0x0100, 0x0101, 0x0102, 0x0103, 0x0104):
            baris, komen = struct.unpack_from('<II', data, i + 8)
            n = {'jenis': j, 'baris': baris, 'komen': st(komen)}
            e = i + kp
            if j in (0x0100, 0x0101):
                a, b = struct.unpack_from('<II', data, e)
                n.update(awalan=st(a), uri=st(b))
            elif j == 0x0103:
                a, b = struct.unpack_from('<II', data, e)
                n.update(ns=st(a), nama=st(b))
            elif j == 0x0104:
                a, _, _, tipe, isi = struct.unpack_from('<IHBBI', data, e)
                n.update(teks=st(a), tipe=tipe, isi=st(isi) if tipe == T_STRING else isi)
            else:
                ns, nama, am, ab, jumlah = struct.unpack_from('<IIHHH', data, e)
                n.update(ns=st(ns), nama=st(nama), atribut=[])
                for k in range(jumlah):
                    a_ns, a_nama, mentah, _, _, tipe, isi = struct.unpack_from('<IIIHBBI', data, e + am + k * ab)
                    n['atribut'].append({'ns': st(a_ns), 'nama': s[a_nama], 'id': peta[a_nama] if a_nama < len(peta) else None,
                                         'mentah': st(mentah), 'tipe': tipe, 'isi': s[isi] if tipe == T_STRING else isi})
            simpul.append(n)
        else:
            raise Galat('potongan manifes tidak dikenal: 0x%04x' % j)
        i += uk
    return simpul, utf8


# ---------- tulis ----------

def tulis_kolam(daftar, utf8):
    isi, ofset = bytearray(), []
    for t in daftar:
        ofset.append(len(isi))
        if utf8:
            b = t.encode('utf-8')
            for n in (len(t), len(b)):
                isi += bytes([n]) if n < 0x80 else bytes([0x80 | (n >> 8), n & 0xff])
            isi += b + b'\0'
        else:
            b = t.encode('utf-16-le')
            n = len(b) // 2
            isi += struct.pack('<H', n) if n < 0x8000 else struct.pack('<HH', 0x8000 | (n >> 16), n & 0xffff)
            isi += b + b'\0\0'
    while len(isi) % 4:
        isi += b'\0'
    kepala = 28
    mulai = kepala + 4 * len(daftar)
    badan = struct.pack('<%dI' % len(daftar), *ofset) + bytes(isi)
    return struct.pack('<HHIIIIII', 0x0001, kepala, kepala + len(badan), len(daftar), 0, 0x100 if utf8 else 0, mulai, 0) + badan


def tulis(simpul, utf8):
    # Nama atribut ber-id harus menempati indeks awal kolam, sejajar dengan peta id sumber daya.
    berid, polos = [], []
    for n in simpul:
        for a in n.get('atribut', []):
            if a['id'] is not None and (a['nama'], a['id']) not in berid:
                berid.append((a['nama'], a['id']))
    indeks_polos = {}

    def p(t):
        if t is None:
            return TANPA
        if t not in indeks_polos:
            indeks_polos[t] = len(berid) + len(polos)
            polos.append(t)
        return indeks_polos[t]

    def nama_attr(a):
        return berid.index((a['nama'], a['id'])) if a['id'] is not None else p(a['nama'])

    keluar = bytearray()
    for n in simpul:
        j = n['jenis']
        if j in (0x0100, 0x0101):
            e = struct.pack('<II', p(n['awalan']), p(n['uri']))
        elif j == 0x0103:
            e = struct.pack('<II', p(n['ns']), p(n['nama']))
        elif j == 0x0104:
            isi = p(n['isi']) if n['tipe'] == T_STRING else n['isi']
            e = struct.pack('<IHBBI', p(n['teks']), 8, 0, n['tipe'], isi)
        else:
            atr = sorted(n['atribut'], key=lambda a: (a['id'] is None, a['id'] or 0))
            e = bytearray()
            for a in atr:
                isi = p(a['isi']) if a['tipe'] == T_STRING else a['isi']
                e += struct.pack('<IIIHBBI', p(a['ns']), nama_attr(a), p(a['mentah']), 8, 0, a['tipe'], isi)
            urut = [a['nama'] if a['ns'] is None else None for a in atr]
            ke = lambda x: urut.index(x) + 1 if x in urut else 0
            e = struct.pack('<IIHHHHHH', p(n['ns']), p(n['nama']), 20, 20, len(atr), ke('id'), ke('class'), ke('style')) + bytes(e)
        keluar += struct.pack('<HHIII', j, 16, 16 + len(e), n['baris'], p(n['komen'])) + e
    kolam = tulis_kolam([a for a, _ in berid] + polos, utf8)
    peta = struct.pack('<HHI', 0x0180, 8, 8 + 4 * len(berid)) + struct.pack('<%dI' % len(berid), *[i for _, i in berid])
    badan = kolam + peta + bytes(keluar)
    return struct.pack('<HHI', 0x0003, 8, 8 + len(badan)) + badan


# ---------- ubah ----------

def atribut(nama, isi):
    tipe = T_BOOL if isinstance(isi, bool) else T_STRING
    return {'ns': NS_ANDROID, 'nama': nama, 'id': ATTR[nama], 'mentah': isi if tipe == T_STRING else None,
            'tipe': tipe, 'isi': (TANPA if isi else 0) if tipe == T_BOOL else isi}


def elemen(nama, baris, attrs):
    return [{'jenis': 0x0102, 'baris': baris, 'komen': None, 'ns': None, 'nama': nama, 'atribut': attrs},
            {'jenis': 0x0103, 'baris': baris, 'komen': None, 'ns': None, 'nama': nama}]


def cari(simpul, nama):
    return [k for k, n in enumerate(simpul) if n['jenis'] == 0x0102 and n['nama'] == nama]


def ubah(simpul, sdk, latar):
    kena = 0
    for n in simpul:
        for a in n.get('atribut', []):
            if a['id'] == ATTR['targetSdkVersion']:
                a.update(tipe=T_DEC, isi=sdk, mentah=None)
                kena += 1
    if kena != 1:
        raise Galat('targetSdkVersion ditemukan %d kali, harusnya sekali' % kena)
    if not latar:
        return simpul
    paket = next(a['isi'] for a in simpul[cari(simpul, 'manifest')[0]]['atribut'] if a['nama'] == 'package')
    akt = cari(simpul, 'activity')
    if len(akt) != 1:
        raise Galat('aktivitas di manifes %d, harusnya satu' % len(akt))
    cc = [a for a in simpul[akt[0]]['atribut'] if a['id'] == ATTR['configChanges']]
    if cc:
        cc[0].update(tipe=T_HEX, isi=cc[0]['isi'] | CONFIG_SEMUA, mentah=None)
    else:
        simpul[akt[0]]['atribut'].append({'ns': NS_ANDROID, 'nama': 'configChanges', 'id': ATTR['configChanges'],
                                          'mentah': None, 'tipe': T_HEX, 'isi': CONFIG_SEMUA})
    ada = {a['isi'] for k in cari(simpul, 'uses-permission') for a in simpul[k]['atribut'] if a['id'] == ATTR['name']}
    app = cari(simpul, 'application')[0]
    baris = simpul[app]['baris']
    izin = []
    for z in IZIN_LATAR:
        if z not in ada:
            izin += elemen('uses-permission', baris, [atribut('name', z)])
    # Akhir <application>: </application> pertama sesudah pembukanya (application tidak bersarang).
    tutup = next(k for k in range(app, len(simpul)) if simpul[k]['jenis'] == 0x0103 and simpul[k]['nama'] == 'application')
    komponen = elemen('service', baris, [atribut('name', 'jrg.ide.latar.Latar$Layanan'), atribut('exported', False)]) + \
        elemen('provider', baris, [atribut('name', 'jrg.ide.latar.Latar$Penyedia'), atribut('authorities', paket + '.latar'),
                                   atribut('exported', False)])
    return simpul[:app] + izin + simpul[app:tutup] + komponen + simpul[tutup:]


def main():
    arg = sys.argv[1:]
    latar = '--latar' in arg
    arg = [a for a in arg if a != '--latar']
    masuk, keluar, nilai = arg[0], arg[1], int(arg[2])
    tambahan = [a.split('=', 1) for a in arg[3:]]
    with zipfile.ZipFile(masuk) as zi, zipfile.ZipFile(keluar, 'w') as zo:
        ada = set(zi.namelist())
        for info in zi.infolist():
            isi = zi.read(info)
            if info.filename == 'AndroidManifest.xml':
                simpul, utf8 = urai(isi)
                isi = tulis(ubah(simpul, nilai, latar), utf8)
            if info.filename.startswith('META-INF/'):
                continue
            baru = zipfile.ZipInfo(info.filename, info.date_time)
            baru.compress_type = info.compress_type
            baru.external_attr = info.external_attr
            zo.writestr(baru, isi)
        for jalur, berkas in tambahan:
            if jalur in ada:
                raise Galat('%s sudah ada di APK' % jalur)
            baru = zipfile.ZipInfo(jalur, (1980, 1, 1, 0, 0, 0))
            baru.compress_type = zipfile.ZIP_STORED
            baru.external_attr = 0o755 << 16
            with open(berkas, 'rb') as f:
                zo.writestr(baru, f.read())


if __name__ == '__main__':
    main()
