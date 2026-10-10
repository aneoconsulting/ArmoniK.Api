; Assembly listing for method Armonik.Ffi.Harness.AbiInit:Run() (Tier0)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier0 code
; rbp based frame
; partially interruptible

G_M000_IG01:                ;; offset=0x0000
       push     rbp
       push     r15
       push     r14
       push     r13
       push     r12
       push     rbx
       sub      rsp, 248
       vzeroupper 
       lea      rbp, [rsp+0x120]
       vxorps   xmm8, xmm8, xmm8
       vmovdqa  xmmword ptr [rbp-0xD0], xmm8
       mov      rcx, -144
       vmovdqa  xmmword ptr [rbp+rcx-0x30], xmm8
       vmovdqa  xmmword ptr [rbp+rcx-0x20], xmm8
       vmovdqa  xmmword ptr [rbp+rcx-0x10], xmm8
       add      rcx, 48
       jne      SHORT  -5 instr
       mov      qword ptr [rbp-0x30], rcx
 
G_M000_IG02:                ;; offset=0x004F
       lea      rdi, [rbp-0x110]
       mov      rsi, r10
       call     CORINFO_HELP_INIT_PINVOKE_FRAME
       mov      qword ptr [rbp-0xA0], rax
       mov      rdi, rsp
       mov      qword ptr [rbp-0xF0], rdi
       mov      rdi, rbp
       mov      qword ptr [rbp-0xE0], rdi
       mov      rdi, ADDR
       call     [System.Environment:GetEnvironmentVariable(System.String):System.String]
       mov      gword ptr [rbp-0xA8], rax
       mov      rdi, gword ptr [rbp-0xA8]
       mov      rsi, ADDR
       call     [System.String:op_Equality(System.String,System.String):bool]
       test     eax, eax
       je       SHORT G_M000_IG03
       mov      edi, -999
       call     [Armonik.Ffi.Harness.AbiInit:set_Code(int)]
       jmp      G_M000_IG09
 
G_M000_IG03:                ;; offset=0x00BB
       vxorps   xmm2, xmm2, xmm2
       vmovdqu  xmmword ptr [rbp-0x68], xmm2
       vmovdqu  xmmword ptr [rbp-0x60], xmm2
       mov      dword ptr [rbp-0x68], 1
       mov      dword ptr [rbp-0x64], 6
       vmovdqu  xmm2, xmmword ptr [rbp-0x68]
       vmovdqu  xmmword ptr [rbp-0x40], xmm2
       mov      rdi, qword ptr [rbp-0x58]
       mov      qword ptr [rbp-0x30], rdi
       xor      edi, edi
       mov      qword ptr [rbp-0x48], rdi
       lea      rdi, [rbp-0x40]
       lea      rsi, [rbp-0x48]
       mov      rax, ADDR
       mov      qword ptr [rbp-0x100], rax
       lea      rax, G_M000_IG05
       mov      qword ptr [rbp-0xE8], rax
       mov      rax, qword ptr [rbp-0xA0]
       lea      rcx, bword ptr [rbp-0x110]
       mov      qword ptr [rax+0x10], rcx
       mov      rax, qword ptr [rbp-0xA0]
       mov      byte  ptr [rax+0x0C], 0
 
G_M000_IG04:                ;; offset=0x0133
       call     [Armonik.Ffi.Harness.Abi:ak_init(ulong,ulong):int]
 
G_M000_IG05:                ;; offset=0x0139
       mov      rcx, qword ptr [rbp-0xA0]
       mov      byte  ptr [rcx+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG06
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG06:                ;; offset=0x0153
       mov      rdi, qword ptr [rbp-0xA0]
       mov      rcx, bword ptr [rbp-0x108]
       mov      qword ptr [rdi+0x10], rcx
       mov      dword ptr [rbp-0x4C], eax
       mov      edi, dword ptr [rbp-0x4C]
       call     [Armonik.Ffi.Harness.AbiInit:set_Code(int)]
       cmp      dword ptr [rbp-0x4C], 0
       je       G_M000_IG08
       cmp      dword ptr [rbp-0x4C], 1
       je       G_M000_IG08
 
G_M000_IG07:                ;; offset=0x0185
       lea      rdi, [rbp-0x90]
       mov      esi, 33
       mov      edx, 3
       call     [System.Runtime.CompilerServices.DefaultInterpolatedStringHandler:.ctor(int,int):this]
       mov      edi, 0x7F84
       mov      rsi, ADDR
       call     CORINFO_HELP_STRCNS
       mov      gword ptr [rbp-0xB0], rax
       mov      rsi, gword ptr [rbp-0xB0]
       lea      rdi, [rbp-0x90]
       call     [System.Runtime.CompilerServices.DefaultInterpolatedStringHandler:AppendLiteral(System.String):this]
       lea      rdi, [rbp-0x90]
       mov      esi, dword ptr [rbp-0x4C]
       call     [System.Runtime.CompilerServices.DefaultInterpolatedStringHandler:AppendFormatted[int](int):this]
       mov      edi, 0x7FA6
       mov      rsi, ADDR
       call     CORINFO_HELP_STRCNS
       mov      gword ptr [rbp-0xB8], rax
       mov      rsi, gword ptr [rbp-0xB8]
       lea      rdi, [rbp-0x90]
       call     [System.Runtime.CompilerServices.DefaultInterpolatedStringHandler:AppendLiteral(System.String):this]
       lea      rdi, [rbp-0x90]
       mov      esi, dword ptr [rbp-0x48]
       call     [System.Runtime.CompilerServices.DefaultInterpolatedStringHandler:AppendFormatted[int](int):this]
       mov      edi, 0x7FB6
       mov      rsi, ADDR
       call     CORINFO_HELP_STRCNS
       mov      gword ptr [rbp-0xC0], rax
       mov      rsi, gword ptr [rbp-0xC0]
       lea      rdi, [rbp-0x90]
       call     [System.Runtime.CompilerServices.DefaultInterpolatedStringHandler:AppendLiteral(System.String):this]
       lea      rdi, [rbp-0x90]
       mov      esi, dword ptr [rbp-0x44]
       call     [System.Runtime.CompilerServices.DefaultInterpolatedStringHandler:AppendFormatted[uint](uint):this]
       mov      edi, 0x7FCA
       mov      rsi, ADDR
       call     CORINFO_HELP_STRCNS
       mov      gword ptr [rbp-0xC8], rax
       mov      rsi, gword ptr [rbp-0xC8]
       lea      rdi, [rbp-0x90]
       call     [System.Runtime.CompilerServices.DefaultInterpolatedStringHandler:AppendLiteral(System.String):this]
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0x98], rax
       lea      rdi, [rbp-0x90]
       call     [System.Runtime.CompilerServices.DefaultInterpolatedStringHandler:ToStringAndClear():System.String:this]
       mov      gword ptr [rbp-0xD0], rax
       mov      rsi, gword ptr [rbp-0xD0]
       mov      rdi, gword ptr [rbp-0x98]
       call     [System.InvalidOperationException:.ctor(System.String):this]
       mov      rdi, gword ptr [rbp-0x98]
       call     CORINFO_HELP_THROW
 
G_M000_IG08:                ;; offset=0x02D2
       jmp      SHORT G_M000_IG09
 
G_M000_IG09:                ;; offset=0x02D4
       add      rsp, 248
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
; Total bytes of code 742

