; Assembly listing for method Armonik.Ffi.Harness.Stage:StrPresent(System.String):Armonik.Ffi.Harness.ak_str:this (Tier1)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1 code
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; partially interruptible
; with Dynamic PGO: edge weights are invalid, and fgCalledCount is 28660
; 2 inlinees with PGO data; 7 single block inlinees; 2 inlinees without PGO data

G_M000_IG01:                ;; offset=0x0000
       push     rbp
       push     r15
       push     r14
       push     r13
       push     r12
       push     rbx
       sub      rsp, 88
       vzeroupper 
       lea      rbp, [rsp+0x80]
       vxorps   xmm8, xmm8, xmm8
       vmovdqa  xmmword ptr [rbp-0x70], xmm8
       vmovdqa  xmmword ptr [rbp-0x60], xmm8
       vmovdqa  xmmword ptr [rbp-0x50], xmm8
       vmovdqa  xmmword ptr [rbp-0x40], xmm8
       xor      eax, eax
       mov      qword ptr [rbp-0x30], rax
       mov      rbx, rdi
       mov      r15, rsi
       mov      r14, rdx
 
G_M000_IG02:                ;; offset=0x0041
       mov      r13d, dword ptr [r14+0x08]
       test     r13d, r13d
       je       G_M000_IG14
       cmp      dword ptr [(reloc ADDR)], 0
       jne      G_M000_IG17
 
G_M000_IG03:                ;; offset=0x005B
       cmp      byte  ptr [rbx+0x44], 0
       jne      G_M000_IG19
       mov      rdi, ADDR
       mov      r12, gword ptr [rdi]
       mov      edi, r13d
       cmp      edi, ADDR
       ja       G_M000_IG21
       lea      eax, [rdi+2*rdi]
       add      eax, 3
       mov      dword ptr [rbp-0x44], eax
       mov      rdi, rbx
       mov      esi, eax
       call     [Armonik.Ffi.Harness.Stage:Room(int):ulong:this]
       mov      qword ptr [rbp-0x50], rax
       add      r14, 12
       mov      bword ptr [rbp-0x58], r14
       mov      r14, bword ptr [rbp-0x58]
       test     r14, r14
       je       G_M000_IG11
 
G_M000_IG04:                ;; offset=0x00AE
       test     rax, rax
       je       G_M000_IG10
 
G_M000_IG05:                ;; offset=0x00B7
       mov      r10d, dword ptr [rbp-0x44]
       mov      r8d, r13d
       or       r8d, r10d
       jl       G_M000_IG22
 
G_M000_IG06:                ;; offset=0x00C7
       lea      r8, [rbp-0x68]
       lea      r9, [rbp-0x70]
       mov      rdi, r14
       mov      esi, r13d
       mov      rdx, rax
       mov      ecx, r10d
       call     [System.Text.Unicode.Utf8Utility:TranscodeToUtf8(ulong,int,ulong,int,byref,byref):int]
       mov      r9, qword ptr [rbp-0x68]
       sub      r9, r14
       mov      rdi, r9
       shr      rdi, 63
       add      r9, rdi
       sar      r9, 1
       mov      rax, qword ptr [rbp-0x70]
       mov      r10, qword ptr [rbp-0x50]
       sub      rax, r10
       cmp      r9d, r13d
       jne      SHORT G_M000_IG09
 
G_M000_IG07:                ;; offset=0x0105
       xor      edx, edx
       mov      bword ptr [rbp-0x58], rdx
       lea      edx, [rax+0x07]
       and      edx, -8
       add      dword ptr [rbx+0x40], edx
       movsxd   r14, eax
       mov      r13, qword ptr [rbx+0x28]
       mov      qword ptr [r15], r10
       mov      qword ptr [r15+0x08], r14
       mov      qword ptr [r15+0x10], r13
       mov      rax, r15
 
G_M000_IG08:                ;; offset=0x0129
       add      rsp, 88
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG09:                ;; offset=0x0138
       mov      dword ptr [rsp], eax
       mov      dword ptr [rsp+0x08], 1
       mov      rdi, r12
       mov      rsi, r14
       mov      edx, r13d
       mov      rcx, r10
       mov      r8d, dword ptr [rbp-0x44]
       call     [System.Text.Encoding:GetBytesWithFallback(ulong,int,ulong,int,int,int,bool):int:this]
       mov      r10, qword ptr [rbp-0x50]
       jmp      SHORT G_M000_IG07
 
G_M000_IG10:                ;; offset=0x015F
       mov      edi, 10
       jmp      SHORT G_M000_IG12
 
G_M000_IG11:                ;; offset=0x0166
       mov      edi, 14
 
G_M000_IG12:                ;; offset=0x016B
       mov      esi, 49
       call     [System.ThrowHelper:ThrowArgumentNullException(int,int)]
       int3     
 
G_M000_IG13:                ;; offset=0x0177
       call     CORINFO_HELP_OVERFLOW
 
G_M000_IG14:                ;; offset=0x017C
       mov      r13, qword ptr [rbx+0x28]
       xor      eax, eax
       mov      qword ptr [r15], rax
 
G_M000_IG15:                ;; offset=0x0185
       mov      qword ptr [r15+0x08], rax
       mov      qword ptr [r15+0x10], r13
       mov      rax, r15
 
G_M000_IG16:                ;; offset=0x0190
       add      rsp, 88
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG17:                ;; offset=0x019F
       cmp      r13d, dword ptr [(reloc ADDR)]
       jl       G_M000_IG03
       cmp      byte  ptr [rbx+0x44], 0
       jne      G_M000_IG03
       lea      rdx, [rbp-0x40]
       mov      rdi, rbx
       mov      rsi, r14
       call     [Armonik.Ffi.Harness.Stage:Alt(System.String,byref):bool:this]
       test     eax, eax
       je       G_M000_IG03
       vmovdqu  xmm0, xmmword ptr [rbp-0x40]
       vmovdqu  xmmword ptr [r15], xmm0
       mov      rax, qword ptr [rbp-0x30]
       mov      qword ptr [r15+0x10], rax
       mov      rax, r15
 
G_M000_IG18:                ;; offset=0x01E3
       add      rsp, 88
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG19:                ;; offset=0x01F2
       lea      r12d, [r13+r13]
       mov      rdi, rbx
       mov      esi, r12d
       call     [Armonik.Ffi.Harness.Stage:Room(int):ulong:this]
       mov      qword ptr [rbp-0x60], rax
       mov      rdi, rbx
       mov      esi, r12d
       call     [Armonik.Ffi.Harness.Stage:Commit(int):this]
       mov      r12, qword ptr [rbp-0x60]
       lea      rdx, bword ptr [r14+0x0C]
       mov      bword ptr [rbp-0x58], rdx
       mov      rsi, bword ptr [rbp-0x58]
       lea      edx, [r13+r13]
       movsxd   rdx, edx
       test     rdx, rdx
       jl       G_M000_IG13
       mov      rdi, r12
       call     [System.Buffer:Memmove(byref,byref,ulong)]
       xor      eax, eax
       mov      bword ptr [rbp-0x58], rax
       mov      r10, r12
       mov      r14d, r13d
       mov      r13, qword ptr [rbx+0x28]
       mov      qword ptr [r15], r10
       mov      qword ptr [r15+0x08], r14
       mov      qword ptr [r15+0x10], r13
       mov      rax, r15
 
G_M000_IG20:                ;; offset=0x025B
       add      rsp, 88
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG21:                ;; offset=0x026A
       call     [System.Text.UTF8Encoding+UTF8EncodingSealed:<GetMaxByteCount>g__ThrowArgumentException|7_0(int)]
       int3     
 
G_M000_IG22:                ;; offset=0x0271
       mov      edi, 12
       mov      esi, 13
       call     [System.ThrowHelper:ThrowArgumentOutOfRangeException(int,int)]
       int3     
 
; Total bytes of code 642

