; Assembly listing for method Armonik.Ffi.Harness.Stage:get_Defer():int (Tier1)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1 code
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; partially interruptible
; with Dynamic PGO: edge weights are valid, and fgCalledCount is 8030

G_M000_IG01:                ;; offset=0x0000
       push     rbp
       mov      rbp, rsp
 
G_M000_IG02:                ;; offset=0x0004
       cmp      dword ptr [(reloc ADDR)], 6
       jne      SHORT G_M000_IG04
       mov      eax, 1
 
G_M000_IG03:                ;; offset=0x0012
       pop      rbp
       ret      
 
G_M000_IG04:                ;; offset=0x0014
       mov      eax, 2
       xor      ecx, ecx
       cmp      dword ptr [(reloc ADDR)], 7
       cmovne   eax, ecx
 
G_M000_IG05:                ;; offset=0x0025
       pop      rbp
       ret      
 
; Total bytes of code 39

