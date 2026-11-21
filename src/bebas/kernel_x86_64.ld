ENTRY(mulai32)
SECTIONS {
    . = 1M;
    __awal_muat = .;
    .multiboot : { KEEP(*(.multiboot)) }
    .text : { *(.text .text.*) }
    .rodata : { *(.rodata .rodata.*) }
    .data : { *(.data .data.*) *(.got .got.plt) }
    .rela.iplt : { __rela_iplt_start = .; *(.rela.iplt) __rela_iplt_end = .; }
    /* TLS (`errno` libm glibc, canary stack-protector): templatnya, disalin ke blok FS saat mulai. */
    .tdata : { __tdata_awal = .; *(.tdata .tdata.* .gnu.linkonce.td.*) __tdata_akhir = .; }
    .tbss : { *(.tbss .tbss.* .gnu.linkonce.tb.*) *(.tcommon) __tbss_akhir = .; }
    __tls_rata = MAX(ALIGNOF(.tdata), ALIGNOF(.tbss));
    .init_array : { __init_awal = .; KEEP(*(SORT(.init_array.*) .init_array)) __init_akhir = .; }
    __akhir_data = .;
    .bss : { *(.bss .bss.*) *(COMMON) }
    . = ALIGN(4096);
    __akhir_bss = .;
    __heap_awal = .;
    /DISCARD/ : { *(.eh_frame*) *(.note*) *(.comment) }
}
