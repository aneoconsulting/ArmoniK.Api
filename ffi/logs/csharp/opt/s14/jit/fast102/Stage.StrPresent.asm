; Assembly listing for method Armonik.Ffi.Harness.Stage:StrPresent(System.String):Armonik.Ffi.Harness.ak_str:this (Tier1)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1 code
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; partially interruptible
; with Dynamic PGO: edge weights are valid, and fgCalledCount is 15826
; 0 inlinees with PGO data; 5 single block inlinees; 1 inlinees without PGO data

G_M000_IG01:                ;; offset=0x0000
       push     rbp
       push     r15
       push     r14
       push     r13
       push     r12
       push     rbx
       sub      rsp, 56
       vzeroupper 
       lea      rbp, [rsp+0x60]
       vxorps   xmm8, xmm8, xmm8
       vmovdqa  xmmword ptr [rbp-0x50], xmm8
       vmovdqa  xmmword ptr [rbp-0x40], xmm8
       xor      eax, eax
       mov      qword ptr [rbp-0x30], rax
       mov      r15, rdi
       mov      rbx, rsi
       mov      r14, rdx
 
G_M000_IG02:                ;; offset=0x0034
       mov      r13d, dword ptr [r14+0x08]
       test     r13d, r13d
       je       G_M000_IG09
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG04
       cmp      r13d, dword ptr [(reloc ADDR)]
       jl       SHORT G_M000_IG04
       cmp      byte  ptr [r15+0x44], 0
       jne      SHORT G_M000_IG04
       lea      rdx, [rbp-0x40]
       mov      rdi, r15
       mov      rsi, r14
       call     [Armonik.Ffi.Harness.Stage:Alt(System.String,byref):bool:this]
       test     eax, eax
       je       SHORT G_M000_IG04
       vmovdqu  xmm0, xmmword ptr [rbp-0x40]
       vmovdqu  xmmword ptr [rbx], xmm0
       mov      rax, qword ptr [rbp-0x30]
       mov      qword ptr [rbx+0x10], rax
       mov      rax, rbx
 
G_M000_IG03:                ;; offset=0x0082
       add      rsp, 56
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG04:                ;; offset=0x0091
       cmp      byte  ptr [r15+0x44], 0
       je       SHORT G_M000_IG06
       lea      r12d, [r13+r13]
       mov      rdi, r15
       mov      esi, r12d
       call     [Armonik.Ffi.Harness.Stage:Room(int):ulong:this]
       mov      qword ptr [rbp-0x58], rax
       mov      rdi, r15
       mov      esi, r12d
       call     [Armonik.Ffi.Harness.Stage:Commit(int):this]
       mov      r12, qword ptr [rbp-0x58]
       add      r14, 12
       mov      bword ptr [rbp-0x50], r14
       mov      rsi, bword ptr [rbp-0x50]
       lea      edx, [r13+r13]
       movsxd   rdx, edx
       test     rdx, rdx
       jl       G_M000_IG08
       mov      rdi, r12
       call     [System.Buffer:Memmove(byref,byref,ulong)]
       xor      eax, eax
       mov      bword ptr [rbp-0x50], rax
       mov      r14d, r13d
       mov      rax, qword ptr [r15+0x28]
       mov      qword ptr [rbx], r12
       mov      qword ptr [rbx+0x08], r14
       mov      qword ptr [rbx+0x10], rax
       mov      rax, rbx
 
G_M000_IG05:                ;; offset=0x00FE
       add      rsp, 56
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG06:                ;; offset=0x010D
       mov      rdi, ADDR
       mov      rdi, gword ptr [rdi]
       mov      esi, r13d
       call     [System.Text.UTF8Encoding+UTF8EncodingSealed:GetMaxByteCount(int):int:this]
       mov      r12d, eax
       mov      rdi, r15
       mov      esi, r12d
       call     [Armonik.Ffi.Harness.Stage:Room(int):ulong:this]
       mov      qword ptr [rbp-0x48], rax
       lea      rdi, bword ptr [r14+0x0C]
       mov      bword ptr [rbp-0x50], rdi
       mov      rsi, bword ptr [rbp-0x50]
       mov      rdi, ADDR
       mov      rdi, gword ptr [rdi]
       mov      edx, r13d
       mov      rcx, rax
       mov      r8d, r12d
       call     [System.Text.UTF8Encoding:GetBytes(ulong,int,ulong,int):int:this]
       mov      r14d, eax
       xor      edi, edi
       mov      bword ptr [rbp-0x50], rdi
       mov      rdi, r15
       mov      esi, r14d
       call     [Armonik.Ffi.Harness.Stage:Commit(int):this]
       mov      r12, qword ptr [rbp-0x48]
       movsxd   r14, r14d
       mov      rax, qword ptr [r15+0x28]
       mov      qword ptr [rbx], r12
       mov      qword ptr [rbx+0x08], r14
       mov      qword ptr [rbx+0x10], rax
       mov      rax, rbx
 
G_M000_IG07:                ;; offset=0x018C
       add      rsp, 56
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG08:                ;; offset=0x019B
       call     CORINFO_HELP_OVERFLOW
 
G_M000_IG09:                ;; offset=0x01A0
       mov      r12, qword ptr [r15+0x28]
       xor      eax, eax
       mov      qword ptr [rbx], rax
 
G_M000_IG10:                ;; offset=0x01A9
       mov      qword ptr [rbx+0x08], rax
       mov      qword ptr [rbx+0x10], r12
       mov      rax, rbx
 
G_M000_IG11:                ;; offset=0x01B4
       add      rsp, 56
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
; Total bytes of code 451

