---
judul: Android apps
ringkas: Packaging a JRG program or project as an APK, from canvas games to apps with a web interface.
---
## From one file

```sh
jrgc game.jrg --apk game.apk                      # for phones (arm64-v8a)
jrgc game.jrg --apk game.apk --apk-abi arm64-v8a,x86_64
adb install -r game.apk
```

## From a project

```sh
jrgm siapkan android      # once: download the platform, build-tools, and NDK
jrgm new mygame --android
cd mygame
jrgm apk                  # target/mygame.apk
jrgm run --android        # install to the phone through adb, open it, follow its output
```

The app's identity is written in `jrgm.toml`:

```toml
[android]
nama       = "My Game"
paket      = "id.jrg.mygame"
versi      = "1.2.0"
orientasi  = "portrait"          # bebas | portrait | landscape
abi        = ["arm64-v8a"]       # add "x86_64" for the emulator
ikon_depan = "aset/ikon.png"     # Android 8+ adaptive icon
ikon_latar = "#0B1230"           # a colour or a PNG
izin       = ["internet", "getar"]
```

## Two kinds of display

| `tampilan` | What it is | Suits |
|---|---|---|
| `"kanvas"` (default) | the program is compiled natively through LLVM and draws full screen | games, drawing, simple tools |
| `"web"` | `jrgc` ships inside the APK and runs your project as a local server; its pages show in a WebView | apps with an HTML interface |

A project from `jrgm new x --kind web` already uses the web display. There the whole language and library are
available, because it is run by the same interpreter as on a computer. The page also gets a `JRGAndroid` object to
ask for permissions (`mintaIzin("kamera")`), vibrate the phone, and exchange files with the phone's storage.

The canvas display uses the LLVM backend, so that backend's limits apply (see [Not done yet](batasan.html)).

## Permissions

`izin = [...]` accepts `internet`, `jaringan` (network), `wifi`, `getar` (vibrate), `tetap_menyala` (keep awake),
`latar` (run in the background with a notification), and the permissions that need a dialog in the web display:
`kamera`, `mikrofon`, `lokasi`, `kontak`, `penyimpanan`, `telepon`, `sms`, `kalender`. A full Android permission
name (`android.permission.X`) is accepted too.

## Languages and native libraries in the APK

```toml
[android]
bahasa = ["lua", "js", "sql"]     # Lua, QuickJS, and SQLite runners ship in the APK

[android.pustaka_c]
libcalc = ["native/calc.c", "native/text.cpp"]

[android.pustaka_rust]
libformula = "rust/formula"
```

`jrgm` downloads the official sources once (their fingerprints are checked), then compiles them for each ABI. A
native library is called from JRG through `extern "C" "libcalc.so" { … }`.

## Releasing to the Play Store

```sh
jrgm kunci                         # once: make a release key
JRG_KUNCI_SANDI=… jrgm apk --rilis
JRG_KUNCI_SANDI=… jrgm apk --aab   # an Android App Bundle for the Play Store
```

An ordinary APK is signed with a debug key. Keep the release key file safe: without the same key, an app update
cannot be installed over the old version.
