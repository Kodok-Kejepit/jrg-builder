# proot untuk JRG IDE Terminal

Biner di `arm64-v8a/` dan `x86_64/` berasal dari mesin Woksole 3.0.0 (pemilik proyek), dibangun
dengan NDK r27c oleh `bangun-proot.sh` dari:

- termux/proot `v5.1.107.96` (commit `a179d3e8a4e045aaa1fb8cc3284f23509d96d353`), GPL-2.0,
  https://github.com/termux/proot, ditambah tiga patch di `patches/`:
  1. `proot-fork-sysexit-new-order-only.patch`: fork tidak lagi crash (`IS_IN_SYSENTER`) di kernel
     dengan urutan seccomp/ptrace lama (regresi proot .94, hasil bisect .93/.94).
  2. `proot-seccomp-legacy-syscalls.patch`: syscall lama yang diblokir seccomp aplikasi Android di
     x86/x86_64 (`readlink`, `fork`, `epoll_create`, ...) diterjemahkan ke versi `*at`/`*1`/`clone`.
  3. `proot-ashmem-memfd-include-string.patch`: include yang hilang untuk clang NDK 27.
- talloc 2.4.2 (LGPL-3.0+), samba.org, ditautkan statis.

| berkas | isi |
|---|---|
| `libproot.so` | proot (executable PIE, nama `lib*.so` supaya Android mengekstraknya ke folder pustaka) |
| `libproot-loader.so` | loader ELF untuk program tamu |
| `libproot-loader32.so` | loader 32 bit (cuma di ABI 64 bit) |

Membangun ulang: `FORCE=1 sh ide/linux/proot/bangun-proot.sh` (sumber diunduh ke `.bangun/`).
