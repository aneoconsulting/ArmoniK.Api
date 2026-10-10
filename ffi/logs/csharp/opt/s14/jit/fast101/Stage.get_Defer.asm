; Assembly listing for method Armonik.Ffi.Harness.Stage:get_Defer():int (Tier1)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1 code
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; partially interruptible
; with Dynamic PGO: edge weights are valid, and fgCalledCount is 26216

G_M000_IG01:                ;; offset=0x0000
       push     rbp
       mov      rbp, rsp
 
G_M000_IG02:                ;; offset=0x0004
       mov      eax, dword ptr [(reloc ADDR)]
       mov      ecx, 1
       mov      edx, 2
       xor      edi, edi
       cmp      eax, 7
       cmovne   edx, edi
       cmp      eax, 6
       mov      eax, edx
       cmove    eax, ecx
 
G_M000_IG03:                ;; offset=0x0024
       pop      rbp
       ret      
 
; Total bytes of code 38

