; Assembly listing for method Armonik.Ffi.Harness.Stage:Room(int):ulong:this (Tier1)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1 code
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; partially interruptible
; with Dynamic PGO: edge weights are valid, and fgCalledCount is 367232
; 0 inlinees with PGO data; 3 single block inlinees; 3 inlinees without PGO data

G_M000_IG01:                ;; offset=0x0000
       push     rbp
       push     r15
       push     r14
       push     r13
       push     r12
       push     rbx
       sub      rsp, 24
       lea      rbp, [rsp+0x40]
       vxorps   xmm8, xmm8, xmm8
       vmovdqa  xmmword ptr [rbp-0x40], xmm8
       xor      eax, eax
       mov      qword ptr [rbp-0x30], rax
       mov      rbx, rdi
       mov      r15d, esi
 
G_M000_IG02:                ;; offset=0x0029
       mov      eax, dword ptr [rbx+0x3C]
       mov      edi, dword ptr [rbx+0x40]
       sub      eax, edi
       cmp      eax, r15d
       jl       SHORT G_M000_IG04
       movsxd   rax, edi
       add      rax, qword ptr [rbx+0x20]
 
G_M000_IG03:                ;; offset=0x003D
       add      rsp, 24
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG04:                ;; offset=0x004C
       mov      r14d, dword ptr [rbx+0x38]
       inc      r14d
       mov      rdi, gword ptr [rbx+0x08]
       cmp      r14d, dword ptr [rdi+0x10]
       jge      SHORT G_M000_IG06
       mov      rdi, gword ptr [rbx+0x10]
       mov      esi, r14d
       cmp      dword ptr [rdi], edi
       call     [System.Collections.Generic.List`1[int]:get_Item(int):int:this]
       cmp      eax, r15d
       jl       SHORT G_M000_IG06
       mov      rdi, rbx
       mov      esi, r14d
       call     [Armonik.Ffi.Harness.Stage:Use(int):this]
       mov      rax, qword ptr [rbx+0x20]
 
G_M000_IG05:                ;; offset=0x0081
       add      rsp, 24
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG06:                ;; offset=0x0090
       xor      r13d, r13d
       mov      rdi, gword ptr [rbx+0x10]
       lea      rsi, [rbp-0x40]
       cmp      dword ptr [rdi], edi
       call     [System.Collections.Generic.List`1[int]:GetEnumerator():System.Collections.Generic.List`1+Enumerator[int]:this]
       jmp      SHORT G_M000_IG10
 
G_M000_IG07:                ;; offset=0x00A5
       mov      edi, dword ptr [rbp-0x30]
       cmp      r13d, edi
       jge      SHORT G_M000_IG08
       jmp      SHORT G_M000_IG09
 
G_M000_IG08:                ;; offset=0x00AF
       mov      edi, r13d
 
G_M000_IG09:                ;; offset=0x00B2
       mov      r13d, edi
 
G_M000_IG10:                ;; offset=0x00B5
       lea      rdi, [rbp-0x40]
       call     [System.Collections.Generic.List`1+Enumerator[int]:MoveNext():bool:this]
       test     eax, eax
       jne      SHORT G_M000_IG07
       movsxd   r12, r15d
       movsxd   rdi, r13d
       add      rdi, rdi
       cmp      r12, rdi
       cmovl    r12, rdi
       cmp      r12, ADDR
       jle      SHORT G_M000_IG11
       mov      edi, ADDR
       cmp      r15d, ADDR
       cmovl    r15d, edi
       movsxd   r12, r15d
 
G_M000_IG11:                ;; offset=0x00EF
       mov      r15, gword ptr [rbx+0x08]
       mov      rdi, r12
       call     [System.Runtime.InteropServices.NativeMemory:Alloc(ulong):ulong]
       mov      rdx, rax
       mov      rdi, r15
       mov      esi, r14d
       cmp      dword ptr [rdi], edi
       call     [System.Collections.Generic.List`1[long]:Insert(int,long):this]
       mov      rdi, gword ptr [rbx+0x10]
       mov      esi, r14d
       mov      edx, r12d
       cmp      dword ptr [rdi], edi
       call     [System.Collections.Generic.List`1[int]:Insert(int,int):this]
       mov      rdi, rbx
       mov      esi, r14d
       call     [Armonik.Ffi.Harness.Stage:Use(int):this]
       mov      rax, qword ptr [rbx+0x20]
 
G_M000_IG12:                ;; offset=0x012F
       add      rsp, 24
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
; Total bytes of code 318

