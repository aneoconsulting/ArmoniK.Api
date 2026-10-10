; Assembly listing for method Armonik.Ffi.Harness.G:E_UploadResultData(byref,Armonik.Ffi.Facade.UploadResultData,Armonik.Ffi.Harness.Stage) (Tier1)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1 code
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; partially interruptible
; with Dynamic PGO: edge weights are valid, and fgCalledCount is 27480
; 2 inlinees with PGO data; 0 single block inlinees; 0 inlinees without PGO data

G_M000_IG01:                ;; offset=0x0000
       push     rbp
       push     r15
       push     r14
       push     rbx
       sub      rsp, 72
       vzeroupper 
       lea      rbp, [rsp+0x60]
       vxorps   xmm8, xmm8, xmm8
       vmovdqa  xmmword ptr [rbp-0x60], xmm8
       vmovdqa  xmmword ptr [rbp-0x50], xmm8
       vmovdqa  xmmword ptr [rbp-0x40], xmm8
       vmovdqa  xmmword ptr [rbp-0x30], xmm8
       xor      eax, eax
       mov      qword ptr [rbp-0x20], rax
       mov      rbx, rdi
       mov      r15, rsi
       mov      r14, rdx
 
G_M000_IG02:                ;; offset=0x003A
       vxorps   ymm0, ymm0, ymm0
       vmovdqu32 zmmword ptr [rbx], zmm0
       vmovdqu  xmmword ptr [rbx+0x40], xmm0
       mov      rdx, gword ptr [r15+0x08]
       cmp      byte  ptr [r14], r14b
       test     rdx, rdx
       je       SHORT G_M000_IG04
 
G_M000_IG03:                ;; offset=0x0055
       cmp      dword ptr [rdx+0x08], 0
       jne      SHORT G_M000_IG05
 
G_M000_IG04:                ;; offset=0x005B
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbp-0x48], xmm0
       vmovdqu  xmmword ptr [rbp-0x40], xmm0
       jmp      SHORT G_M000_IG06
 
G_M000_IG05:                ;; offset=0x006B
       lea      rsi, [rbp-0x48]
       mov      rdi, r14
       call     [Armonik.Ffi.Harness.Stage:StrPresent(System.String):Armonik.Ffi.Harness.ak_str:this]
 
G_M000_IG06:                ;; offset=0x0078
       vmovdqu  xmm0, xmmword ptr [rbp-0x48]
       vmovdqu  xmmword ptr [rbx], xmm0
       mov      rsi, qword ptr [rbp-0x38]
       mov      qword ptr [rbx+0x10], rsi
       mov      rdx, gword ptr [r15+0x10]
       test     rdx, rdx
       je       SHORT G_M000_IG08
 
G_M000_IG07:                ;; offset=0x0092
       cmp      dword ptr [rdx+0x08], 0
       jne      SHORT G_M000_IG09
 
G_M000_IG08:                ;; offset=0x0098
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbp-0x60], xmm0
       vmovdqu  xmmword ptr [rbp-0x58], xmm0
       jmp      SHORT G_M000_IG10
 
G_M000_IG09:                ;; offset=0x00A8
       lea      rsi, [rbp-0x60]
       mov      rdi, r14
       call     [Armonik.Ffi.Harness.Stage:StrPresent(System.String):Armonik.Ffi.Harness.ak_str:this]
 
G_M000_IG10:                ;; offset=0x00B5
       vmovdqu  xmm0, xmmword ptr [rbp-0x60]
       vmovdqu  xmmword ptr [rbx+0x18], xmm0
       mov      rax, qword ptr [rbp-0x50]
       mov      qword ptr [rbx+0x28], rax
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbp-0x30], xmm0
       vmovdqu  xmmword ptr [rbp-0x28], xmm0
       mov      qword ptr [rbp-0x30], 1
       lea      rax, bword ptr [rbp-0x30]
       mov      rcx, gword ptr [r15+0x18]
       test     rcx, rcx
       je       SHORT G_M000_IG13
       mov      ecx, dword ptr [rcx+0x08]
 
G_M000_IG11:                ;; offset=0x00ED
       movsxd   rcx, ecx
       mov      qword ptr [rax+0x08], rcx
       xor      eax, eax
       mov      qword ptr [rbp-0x20], rax
       vmovdqu  xmm0, xmmword ptr [rbp-0x30]
       vmovdqu  xmmword ptr [rbx+0x30], xmm0
       mov      rax, qword ptr [rbp-0x20]
       mov      qword ptr [rbx+0x40], rax
 
G_M000_IG12:                ;; offset=0x010C
       add      rsp, 72
       pop      rbx
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG13:                ;; offset=0x0117
       xor      ecx, ecx
       jmp      SHORT G_M000_IG11
 
; Total bytes of code 283

