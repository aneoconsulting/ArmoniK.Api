; Assembly listing for method Armonik.Ffi.Harness.G:E_UploadResultDataMessage(byref,Armonik.Ffi.Facade.UploadResultDataMessage,Armonik.Ffi.Harness.Stage) (Tier1)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1 code
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; partially interruptible
; with Dynamic PGO: edge weights are valid, and fgCalledCount is 49688
; 3 inlinees with PGO data; 0 single block inlinees; 0 inlinees without PGO data

G_M000_IG01:                ;; offset=0x0000
       push     rbp
       push     r15
       push     r14
       push     rbx
       sub      rsp, 72
       vzeroupper 
       lea      rbp, [rsp+0x60]
       mov      rbx, rdi
       mov      r15, rdx
 
G_M000_IG02:                ;; offset=0x0018
       vxorps   ymm0, ymm0, ymm0
       vmovdqu32 zmmword ptr [rbx], zmm0
       vmovdqu  ymmword ptr [rbx+0x38], ymm0
       mov      r14, gword ptr [rsi+0x08]
       test     r14, r14
       je       G_M000_IG11
 
G_M000_IG03:                ;; offset=0x0034
       mov      rsi, rbx
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbp-0x30], xmm0
       vmovdqu  xmmword ptr [rbp-0x28], xmm0
       vxorps   ymm0, ymm0, ymm0
       vmovdqu32 zmmword ptr [rsi], zmm0
       vmovdqu  xmmword ptr [rsi+0x40], xmm0
       mov      rdx, gword ptr [r14+0x08]
       cmp      byte  ptr [r15], r15b
       test     rdx, rdx
       je       SHORT G_M000_IG05
 
G_M000_IG04:                ;; offset=0x0060
       cmp      dword ptr [rdx+0x08], 0
       jne      G_M000_IG12
 
G_M000_IG05:                ;; offset=0x006A
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbp-0x48], xmm0
       vmovdqu  xmmword ptr [rbp-0x40], xmm0
 
G_M000_IG06:                ;; offset=0x0078
       vmovdqu  xmm0, xmmword ptr [rbp-0x48]
       vmovdqu  xmmword ptr [rbx], xmm0
       mov      rsi, qword ptr [rbp-0x38]
       mov      qword ptr [rbx+0x10], rsi
       mov      rdx, gword ptr [r14+0x10]
       test     rdx, rdx
       je       SHORT G_M000_IG08
 
G_M000_IG07:                ;; offset=0x0092
       cmp      dword ptr [rdx+0x08], 0
       jne      G_M000_IG13
 
G_M000_IG08:                ;; offset=0x009C
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbp-0x60], xmm0
       vmovdqu  xmmword ptr [rbp-0x58], xmm0
 
G_M000_IG09:                ;; offset=0x00AA
       vmovdqu  xmm0, xmmword ptr [rbp-0x60]
       vmovdqu  xmmword ptr [rbx+0x18], xmm0
       mov      rax, qword ptr [rbp-0x50]
       mov      qword ptr [rbx+0x28], rax
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbp-0x30], xmm0
       vmovdqu  xmmword ptr [rbp-0x28], xmm0
       mov      qword ptr [rbp-0x30], 1
       lea      rax, bword ptr [rbp-0x30]
       mov      rcx, gword ptr [r14+0x18]
       test     rcx, rcx
       je       SHORT G_M000_IG14
       mov      ecx, dword ptr [rcx+0x08]
 
G_M000_IG10:                ;; offset=0x00E2
       movsxd   rcx, ecx
       mov      qword ptr [rax+0x08], rcx
       xor      eax, eax
       mov      qword ptr [rbp-0x20], rax
       vmovdqu  xmm0, xmmword ptr [rbp-0x30]
       vmovdqu  xmmword ptr [rbx+0x30], xmm0
       mov      rax, qword ptr [rbp-0x20]
       mov      qword ptr [rbx+0x40], rax
       add      rbx, 80
       or       dword ptr [rbx], 1
 
G_M000_IG11:                ;; offset=0x0108
       add      rsp, 72
       pop      rbx
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG12:                ;; offset=0x0113
       lea      rsi, [rbp-0x48]
       mov      rdi, r15
       call     [Armonik.Ffi.Harness.Stage:StrPresent(System.String):Armonik.Ffi.Harness.ak_str:this]
       jmp      G_M000_IG06
 
G_M000_IG13:                ;; offset=0x0125
       lea      rsi, [rbp-0x60]
       mov      rdi, r15
       call     [Armonik.Ffi.Harness.Stage:StrPresent(System.String):Armonik.Ffi.Harness.ak_str:this]
       jmp      G_M000_IG09
 
G_M000_IG14:                ;; offset=0x0137
       xor      ecx, ecx
       jmp      SHORT G_M000_IG10
 
; Total bytes of code 315

