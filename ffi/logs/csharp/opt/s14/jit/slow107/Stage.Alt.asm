; Assembly listing for method Armonik.Ffi.Harness.Stage:Alt(System.String,byref):bool:this (Tier1)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1 code
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; partially interruptible
; with Dynamic PGO: edge weights are valid, and fgCalledCount is 479
; 0 inlinees with PGO data; 5 single block inlinees; 1 inlinees without PGO data

G_M000_IG01:                ;; offset=0x0000
       push     rbp
       push     r15
       push     r14
       push     r13
       push     r12
       push     rbx
       sub      rsp, 24
       vzeroupper 
       lea      rbp, [rsp+0x40]
       vxorps   xmm8, xmm8, xmm8
       vmovdqa  xmmword ptr [rbp-0x40], xmm8
       xor      eax, eax
       mov      qword ptr [rbp-0x30], rax
       mov      r14, rdi
       mov      r15, rsi
       mov      rbx, rdx
 
G_M000_IG02:                ;; offset=0x002F
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbx], xmm0
       vmovdqu  xmmword ptr [rbx+0x08], xmm0
       mov      r13d, dword ptr [(reloc ADDR)]
       cmp      byte  ptr [(reloc ADDR)], 0
       jne      G_M000_IG08
 
G_M000_IG03:                ;; offset=0x0050
       lea      r12d, [r13-0x01]
       cmp      r12d, 4
       ja       SHORT G_M000_IG04
       mov      edi, r12d
       lea      rsi, [reloc @RWD00]
       mov      esi, dword ptr [rsi+4*rdi]
       lea      rax, G_M000_IG02
       add      rsi, rax
       jmp      rsi
 
G_M000_IG04:                ;; offset=0x0073
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax], 2
       jl       G_M000_IG16
       mov      rax, qword ptr [rax+0x08]
       mov      rax, qword ptr [rax+0x10]
       test     rax, rax
       je       G_M000_IG16
 
G_M000_IG05:                ;; offset=0x00A3
       inc      qword ptr [rax+0x128]
       mov      r14, ADDR
       cmp      qword ptr [r14], 0
       je       G_M000_IG17
 
G_M000_IG06:                ;; offset=0x00BE
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbp-0x40], xmm0
       vmovdqu  xmmword ptr [rbp-0x38], xmm0
       mov      qword ptr [rbp-0x40], 0x30000
       lea      rax, bword ptr [rbp-0x40]
       mov      edi, dword ptr [r15+0x08]
       mov      qword ptr [rax+0x08], rdi
       mov      rax, qword ptr [r14]
       mov      qword ptr [rbp-0x30], rax
       vmovdqu  xmm0, xmmword ptr [rbp-0x40]
       vmovdqu  xmmword ptr [rbx], xmm0
       mov      rax, qword ptr [rbp-0x30]
       mov      qword ptr [rbx+0x10], rax
       mov      eax, 1
 
G_M000_IG07:                ;; offset=0x00FD
       add      rsp, 24
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG08:                ;; offset=0x010C
       test     r15, r15
       jne      SHORT G_M000_IG09
       xor      rdi, rdi
       xor      esi, esi
       jmp      SHORT G_M000_IG10
 
G_M000_IG09:                ;; offset=0x0117
       lea      rdi, bword ptr [r15+0x0C]
       mov      esi, dword ptr [r15+0x08]
 
G_M000_IG10:                ;; offset=0x011F
       call     [System.Text.Ascii:IsValidCore[ushort](byref,int):bool]
       test     eax, eax
       je       G_M000_IG03
       xor      eax, eax
 
G_M000_IG11:                ;; offset=0x012F
       add      rsp, 24
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG12:                ;; offset=0x013E
       mov      rdi, r14
       mov      rsi, rbx
       mov      rdx, r15
       call     [Armonik.Ffi.Harness.Stage:Pin(System.String):Armonik.Ffi.Harness.ak_str:this]
       mov      eax, 1
 
G_M000_IG13:                ;; offset=0x0152
       add      rsp, 24
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG14:                ;; offset=0x0161
       mov      rdi, rbx
       mov      rsi, r15
       mov      edx, r13d
       call     [Armonik.Ffi.Harness.Stage:Tab(System.String,int):Armonik.Ffi.Harness.ak_str]
       mov      eax, 1
 
G_M000_IG15:                ;; offset=0x0175
       add      rsp, 24
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG16:                ;; offset=0x0184
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG05
 
G_M000_IG17:                ;; offset=0x0193
       call     Armonik.Ffi.Harness.Abi:ak_tc_utf16():long
       mov      qword ptr [r14], rax
       jmp      G_M000_IG06
 
RWD00  	dd	0000010Fh ; case G_M000_IG12
       	dd	00000132h ; case G_M000_IG14
       	dd	0000010Fh ; case G_M000_IG12
       	dd	00000132h ; case G_M000_IG14
       	dd	00000132h ; case G_M000_IG14

; Total bytes of code 416

