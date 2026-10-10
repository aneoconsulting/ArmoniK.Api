; Assembly listing for method Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinR_e(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long (Tier1)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1 code
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; partially interruptible
; with Dynamic PGO: edge weights are valid, and fgCalledCount is 845
; 0 inlinees with PGO data; 2 single block inlinees; 0 inlinees without PGO data

G_M000_IG01:                ;; offset=0x0000
       push     rbp
       push     r15
       push     r14
       push     r13
       push     r12
       push     rbx
       sub      rsp, 120
       lea      rbp, [rsp+0xA0]
       vxorps   xmm8, xmm8, xmm8
       vmovdqu  xmmword ptr [rbp-0x48], xmm8
       xor      ebx, ebx
       mov      qword ptr [rbp-0x38], rbx
       mov      qword ptr [rbp-0x30], rdx
       mov      r13, rdi
       mov      r12, rsi
       mov      rbx, rcx
       mov      r14, r8
       mov      r15, r9
 
G_M000_IG02:                ;; offset=0x0039
       lea      rdi, [rbp-0x90]
       mov      rsi, r10
       call     CORINFO_HELP_INIT_PINVOKE_FRAME
       mov      qword ptr [rbp-0x50], rax
       mov      rdi, rsp
       mov      qword ptr [rbp-0x70], rdi
       mov      rdi, rbp
       mov      qword ptr [rbp-0x60], rdi
       mov      rdi, gword ptr [r14+0x08]
       mov      r14, rdi
       test     rdi, rdi
       je       G_M000_IG17
       mov      rdi, gword ptr [r14+0x08]
 
G_M000_IG03:                ;; offset=0x006E
       test     rdi, rdi
       je       G_M000_IG18
       add      rdi, 12
       mov      bword ptr [rbp-0x38], rdi
       mov      rcx, bword ptr [rbp-0x38]
 
G_M000_IG04:                ;; offset=0x0083
       test     r14, r14
       je       G_M000_IG19
       mov      r14, gword ptr [r14+0x10]
       test     r14, r14
       je       G_M000_IG20
 
G_M000_IG05:                ;; offset=0x0099
       add      r14, 12
       mov      bword ptr [rbp-0x40], r14
       mov      r14, bword ptr [rbp-0x40]
 
G_M000_IG06:                ;; offset=0x00A5
       cmp      qword ptr [rbx], 0x30000
       jne      SHORT G_M000_IG08
       mov      qword ptr [rbx], rcx
       mov      rdi, ADDR
       mov      rcx, ADDR
       call     rcx
       cmp      dword ptr [rax], 2
       jl       G_M000_IG21
       mov      r9, qword ptr [rax+0x08]
       mov      rax, qword ptr [r9+0x10]
       test     rax, rax
       je       G_M000_IG21
 
G_M000_IG07:                ;; offset=0x00E1
       inc      qword ptr [rax+0x130]
 
G_M000_IG08:                ;; offset=0x00E8
       cmp      qword ptr [rbx+0x18], 0x30000
       je       G_M000_IG22
 
G_M000_IG09:                ;; offset=0x00F6
       mov      gword ptr [rbp-0x48], r15
       test     r15, r15
       je       SHORT G_M000_IG10
       mov      r9, gword ptr [rbp-0x48]
       cmp      dword ptr [r9+0x08], 0
       jne      G_M000_IG23
 
G_M000_IG10:                ;; offset=0x010E
       xor      r8d, r8d
 
G_M000_IG11:                ;; offset=0x0111
       mov      r9d, dword ptr [r15+0x08]
       mov      rdi, r13
       mov      rsi, r12
       mov      rdx, qword ptr [rbp-0x30]
       mov      rcx, rbx
       mov      r10, ADDR
       mov      qword ptr [rbp-0x80], r10
       lea      r10, G_M000_IG14
       mov      qword ptr [rbp-0x68], r10
       lea      r10, bword ptr [rbp-0x90]
       mov      rax, qword ptr [rbp-0x50]
       mov      qword ptr [rax+0x10], r10
       mov      byte  ptr [rax+0x0C], 0
 
G_M000_IG12:                ;; offset=0x014E
       mov      r10, ADDR
 
G_M000_IG13:                ;; offset=0x0158
       call     r10 ; Armonik.Ffi.Harness.Abi:ak_encode_UploadResultDataMessage(ulong,long,ulong,ulong,ulong,ulong):long
 
G_M000_IG14:                ;; offset=0x015B
       mov      r15, qword ptr [rbp-0x50]
       mov      byte  ptr [r15+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG15
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG15:                ;; offset=0x0173
       mov      rdi, bword ptr [rbp-0x88]
       mov      qword ptr [r15+0x10], rdi
 
G_M000_IG16:                ;; offset=0x017E
       add      rsp, 120
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG17:                ;; offset=0x018D
       xor      r14, r14
       xor      rdi, rdi
       jmp      G_M000_IG03
 
G_M000_IG18:                ;; offset=0x0197
       xor      ecx, ecx
       jmp      G_M000_IG04
 
G_M000_IG19:                ;; offset=0x019E
       xor      r14, r14
       test     r14, r14
       jne      G_M000_IG05
 
G_M000_IG20:                ;; offset=0x01AA
       xor      r14d, r14d
       jmp      G_M000_IG06
 
G_M000_IG21:                ;; offset=0x01B2
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG07
 
G_M000_IG22:                ;; offset=0x01C1
       mov      qword ptr [rbx+0x18], r14
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       inc      qword ptr [rax+0x130]
       jmp      G_M000_IG09
 
G_M000_IG23:                ;; offset=0x01DB
       mov      rcx, gword ptr [rbp-0x48]
       cmp      dword ptr [rcx+0x08], 0
       jbe      SHORT G_M000_IG24
       mov      r8, gword ptr [rbp-0x48]
       add      r8, 16
       jmp      G_M000_IG11
 
G_M000_IG24:                ;; offset=0x01F2
       call     CORINFO_HELP_RNGCHKFAIL
       int3     
 
; Total bytes of code 504

