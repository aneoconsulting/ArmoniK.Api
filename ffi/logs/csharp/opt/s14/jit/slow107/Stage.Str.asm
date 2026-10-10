; Assembly listing for method Armonik.Ffi.Harness.Stage:Str(System.String):Armonik.Ffi.Harness.ak_str:this (Tier1)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1 code
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; partially interruptible
; with Dynamic PGO: edge weights are valid, and fgCalledCount is 28306

G_M000_IG01:                ;; offset=0x0000
       push     rbp
       push     rbx
       push     rax
       vzeroupper 
       lea      rbp, [rsp+0x10]
       mov      rbx, rsi
 
G_M000_IG02:                ;; offset=0x000E
       test     rdx, rdx
       je       SHORT G_M000_IG04
 
G_M000_IG03:                ;; offset=0x0013
       cmp      dword ptr [rdx+0x08], 0
       jne      SHORT G_M000_IG06
 
G_M000_IG04:                ;; offset=0x0019
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbx], xmm0
       vmovdqu  xmmword ptr [rbx+0x08], xmm0
       mov      rax, rbx
 
G_M000_IG05:                ;; offset=0x0029
       add      rsp, 8
       pop      rbx
       pop      rbp
       ret      
 
G_M000_IG06:                ;; offset=0x0030
       mov      rsi, rbx
       call     [Armonik.Ffi.Harness.Stage:StrPresent(System.String):Armonik.Ffi.Harness.ak_str:this]
       mov      rax, rbx
 
G_M000_IG07:                ;; offset=0x003C
       add      rsp, 8
       pop      rbx
       pop      rbp
       ret      
 
; Total bytes of code 67

