// Boot stub: multiboot1 (QEMU -kernel / GRUB) -> long mode 64 bit -> jrg_bebas_mulai.
    .set MB_MAGIC, 0x1BADB002
    .set MB_FLAGS, 0x00010003          // align, meminfo, alamat dari kepala (a.out kludge: ELF64 dimuat apa adanya)
    .section .multiboot, "a"
    .align 4
mb_kepala:
    .long MB_MAGIC
    .long MB_FLAGS
    .long -(MB_MAGIC + MB_FLAGS)
    .long mb_kepala                    // header_addr
    .long __awal_muat                  // load_addr
    .long __akhir_data                 // load_end_addr
    .long __akhir_bss                  // bss_end_addr
    .long mulai32                      // entry_addr

    .section .bss
    .align 4096
    .global pml4, pdpt, pd
pml4:   .skip 4096
pdpt:   .skip 4096
pd:     .skip 4096 * 4
    // Halaman 2 MiB tepat di bawah stack utama dilepas dari peta (`libc_mini.c`): stack habis = page fault di sini.
    .align 0x200000
    .global jrg_bebas_penjaga
jrg_bebas_penjaga: .skip 0x200000
    .global jrg_bebas_stack_bawah
jrg_bebas_stack_bawah: .skip 8 * 1024 * 1024
stack_atas:

    .section .data
    .align 16
gdt64:
    .quad 0
    .quad 0x00AF9A000000FFFF           // kode 64 bit
    .quad 0x00CF92000000FFFF           // data
    .global jrg_bebas_gdt_tss
jrg_bebas_gdt_tss:                     // selektor 0x18
    .quad 0, 0                         // TSS (diisi C)
gdt64_akhir:
    .global jrg_bebas_gdtr
jrg_bebas_gdtr:
    .word gdt64_akhir - gdt64 - 1
    .quad gdt64
mb_info: .long 0

    .section .text
    .code32
    .global mulai32
mulai32:
    cli
    mov $stack_atas, %esp
    mov %ebx, mb_info
    // PML4[0] -> PDPT, PDPT[0..3] -> PD 0..3, tiap PD 512 halaman 2 MiB: identitas 4 GiB.
    mov $pdpt, %eax
    or $3, %eax
    mov %eax, pml4
    mov $0, %ecx
1:  mov %ecx, %eax
    shl $12, %eax
    add $pd, %eax
    or $3, %eax
    mov %eax, pdpt(,%ecx,8)
    inc %ecx
    cmp $4, %ecx
    jne 1b
    mov $0, %ecx
2:  mov %ecx, %eax
    shl $21, %eax
    or $0x83, %eax
    mov %eax, pd(,%ecx,8)
    mov %ecx, %eax
    shr $11, %eax
    mov %eax, pd+4(,%ecx,8)
    inc %ecx
    cmp $2048, %ecx
    jne 2b
    mov $pml4, %eax
    mov %eax, %cr3
    mov %cr4, %eax
    or $0x6A0, %eax                    // PAE, PGE, OSFXSR, OSXMMEXCPT
    mov %eax, %cr4
    mov $0xC0000080, %ecx
    rdmsr
    or $0x100, %eax                    // LME
    wrmsr
    mov %cr0, %eax
    and $0xFFFFFFFB, %eax              // tanpa EM
    or $0x80000002, %eax               // PG, MP
    mov %eax, %cr0
    lgdt jrg_bebas_gdtr
    ljmp $0x08, $mulai64

    .code64
mulai64:
    mov $0x10, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %ss
    xor %ax, %ax
    mov %ax, %fs
    mov %ax, %gs
    mov $stack_atas, %rsp
    xor %rbp, %rbp
    mov mb_info(%rip), %edi
    call jrg_bebas_mulai
3:  cli
    hlt
    jmp 3b

// ── 256 pintu interupsi: tiap pintu menyamakan bingkai (kode galat 0 kalau CPU tidak mendorongnya) ──
    .macro pintu n
    .p2align 4
pintu_\n:
    .if (\n == 8) || ((\n >= 10) && (\n <= 14)) || (\n == 17) || (\n == 21) || (\n == 29) || (\n == 30)
    .else
    pushq $0
    .endif
    pushq $\n
    jmp pintu_umum
    .endm

    .altmacro
    .set i, 0
    .rept 256
    pintu %i
    .set i, i + 1
    .endr

    .macro alamat_pintu n
    .quad pintu_\n
    .endm
    .section .rodata
    .global jrg_bebas_pintu
    .p2align 3
jrg_bebas_pintu:
    .set i, 0
    .rept 256
    alamat_pintu %i
    .set i, i + 1
    .endr
    .noaltmacro

    .section .text
pintu_umum:
    push %rax
    push %rbx
    push %rcx
    push %rdx
    push %rsi
    push %rdi
    push %rbp
    push %r8
    push %r9
    push %r10
    push %r11
    push %r12
    push %r13
    push %r14
    push %r15
    mov %rsp, %rdi                     // bingkai (`JrgBingkai` di libc_mini.c)
    mov %rsp, %rbx
    and $-64, %rsp
    sub $4096, %rsp                    // keadaan SSE/AVX: xsave kalau AVX aktif, selain itu fxsave
    cmpb $0, jrg_bebas_pakai_xsave(%rip)
    je 1f
    mov $-1, %eax
    mov $-1, %edx
    xsave (%rsp)
    jmp 2f
1:  fxsave (%rsp)
2:  cld
    call jrg_bebas_interupsi
    cmpb $0, jrg_bebas_pakai_xsave(%rip)
    je 3f
    mov $-1, %eax
    mov $-1, %edx
    xrstor (%rsp)
    jmp 4f
3:  fxrstor (%rsp)
4:  mov %rbx, %rsp
    pop %r15
    pop %r14
    pop %r13
    pop %r12
    pop %r11
    pop %r10
    pop %r9
    pop %r8
    pop %rbp
    pop %rdi
    pop %rsi
    pop %rdx
    pop %rcx
    pop %rbx
    pop %rax
    add $16, %rsp
    iretq
