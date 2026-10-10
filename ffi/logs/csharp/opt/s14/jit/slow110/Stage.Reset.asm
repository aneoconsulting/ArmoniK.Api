; Assembly listing for method Armonik.Ffi.Harness.Stage:Reset():this (Tier1)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1 code
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; fully interruptible
; with Dynamic PGO: edge weights are valid, and fgCalledCount is 22060
; 4 inlinees with PGO data; 4 single block inlinees; 2 inlinees without PGO data

G_M000_IG01:                ;; offset=0x0000
       push     rbp
       push     r15
       push     r14
       push     rbx
       push     rax
       lea      rbp, [rsp+0x20]
       mov      rbx, rdi
 
G_M000_IG02:                ;; offset=0x000F
       xor      edi, edi
       mov      dword ptr [rbx+0x38], edi
       mov      rdi, gword ptr [rbx+0x08]
       cmp      dword ptr [rdi+0x10], 0
       jbe      G_M000_IG11
       mov      rdi, gword ptr [rdi+0x08]
       cmp      dword ptr [rdi+0x08], 0
       jbe      G_M000_IG16
       mov      rdi, qword ptr [rdi+0x10]
       mov      qword ptr [rbx+0x20], rdi
       mov      rdi, gword ptr [rbx+0x10]
       cmp      dword ptr [rdi+0x10], 0
       jbe      G_M000_IG11
       mov      rdi, gword ptr [rdi+0x08]
       cmp      dword ptr [rdi+0x08], 0
       jbe      G_M000_IG16
       mov      edi, dword ptr [rdi+0x10]
       mov      dword ptr [rbx+0x3C], edi
       xor      edi, edi
       mov      dword ptr [rbx+0x40], edi
 
G_M000_IG03:                ;; offset=0x005F
       mov      qword ptr [rbp-0x20], rdi
 
G_M000_IG04:                ;; offset=0x0063
       xor      r15d, r15d
 
G_M000_IG05:                ;; offset=0x0066
       mov      r14, gword ptr [rbx+0x18]
       cmp      r15d, dword ptr [r14+0x10]
       jl       SHORT G_M000_IG10
 
G_M000_IG06:                ;; offset=0x0070
       inc      dword ptr [r14+0x14]
       xor      edi, edi
       mov      dword ptr [r14+0x10], edi
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax+0x10], 3
       jl       SHORT G_M000_IG13
 
G_M000_IG07:                ;; offset=0x0096
       mov      rax, qword ptr [rax+0x18]
       mov      rax, qword ptr [rax+0x18]
       test     rax, rax
       je       SHORT G_M000_IG13
       mov      rax, bword ptr [rax]
       add      rax, 16
       mov      r14, gword ptr [rax+0x08]
       test     r14, r14
       jne      SHORT G_M000_IG14
 
G_M000_IG08:                ;; offset=0x00B3
       cmp      dword ptr [(reloc ADDR)], 7
       je       G_M000_IG15
 
G_M000_IG09:                ;; offset=0x00C0
       add      rsp, 8
       pop      rbx
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG10:                ;; offset=0x00CB
       mov      rax, r14
       cmp      r15d, dword ptr [rax+0x10]
       jb       SHORT G_M000_IG12
 
G_M000_IG11:                ;; offset=0x00D4
       call     [System.ThrowHelper:ThrowArgumentOutOfRange_IndexMustBeLessException()]
       int3     
 
G_M000_IG12:                ;; offset=0x00DB
       mov      rdi, gword ptr [rax+0x08]
       cmp      r15d, dword ptr [rdi+0x08]
       jae      SHORT G_M000_IG16
       mov      eax, r15d
       mov      rdi, qword ptr [rdi+8*rax+0x10]
       mov      qword ptr [rbp-0x20], rdi
       lea      rdi, [rbp-0x20]
       call     [System.Runtime.InteropServices.GCHandle:Free():this]
       inc      r15d
       jmp      G_M000_IG05
 
G_M000_IG13:                ;; offset=0x0103
       mov      edi, 3
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      r14, gword ptr [rax+0x08]
       test     r14, r14
       je       SHORT G_M000_IG08
 
G_M000_IG14:                ;; offset=0x0116
       inc      dword ptr [r14+0x14]
       mov      edx, dword ptr [r14+0x10]
       xor      edi, edi
       mov      dword ptr [r14+0x10], edi
       test     edx, edx
       jle      SHORT G_M000_IG08
       mov      rdi, gword ptr [r14+0x08]
       xor      esi, esi
       call     [System.Array:Clear(System.Array,int,int)]
       cmp      dword ptr [(reloc ADDR)], 7
       jne      SHORT G_M000_IG09
 
G_M000_IG15:                ;; offset=0x013D
       xor      edi, edi
       call     [Armonik.Ffi.Harness.Stage:ReleaseChunk(int)]
       jmp      G_M000_IG09
 
G_M000_IG16:                ;; offset=0x014A
       call     CORINFO_HELP_RNGCHKFAIL
       int3     
 
; Total bytes of code 336

