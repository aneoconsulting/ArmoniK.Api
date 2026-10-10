; Assembly listing for method Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:Go(Armonik.Ffi.Facade.UploadResultDataMessage,bool,bool,byref,byref):int:this (Tier1)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1 code
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; partially interruptible
; with Dynamic PGO: edge weights are invalid, and fgCalledCount is 26772
; 12 inlinees with PGO data; 7 single block inlinees; 3 inlinees without PGO data

G_M000_IG01:                ;; offset=0x0000
       push     rbp
       push     r15
       push     r14
       push     r13
       push     r12
       push     rbx
       sub      rsp, 504
       vzeroupper 
       lea      rbp, [rsp+0x220]
       vxorps   xmm8, xmm8, xmm8
       mov      rbx, -336
       vmovdqa  xmmword ptr [rbp+rbx-0x40], xmm8
       vmovdqa  xmmword ptr [rbp+rbx-0x30], xmm8
       vmovdqa  xmmword ptr [rbp+rbx-0x20], xmm8
       add      rbx, 48
       jne      SHORT  -5 instr
       mov      qword ptr [rbp-0x40], rbx
       mov      gword ptr [rbp-0x1A8], rsi
       mov      bword ptr [rbp-0x1B8], r9
       mov      rbx, rdi
       mov      r15d, edx
       mov      r13d, ecx
       mov      r12, r8
 
G_M000_IG02:                ;; offset=0x0061
       lea      rdi, [rbp-0x210]
       mov      rsi, r10
       call     CORINFO_HELP_INIT_PINVOKE_FRAME
       mov      qword ptr [rbp-0x198], rax
       mov      rdi, rsp
       mov      qword ptr [rbp-0x1F0], rdi
       mov      rdi, rbp
       mov      qword ptr [rbp-0x1E0], rdi
       xor      edi, edi
       mov      bword ptr [rbp-0x1B0], r12
       mov      qword ptr [r12], rdi
 
G_M000_IG03:                ;; offset=0x0098
       mov      rcx, bword ptr [rbp-0x1B8]
       mov      dword ptr [rcx], edi
       mov      gword ptr [rbp-0x1A0], rbx
       mov      rdi, qword ptr [rbx+0x18]
       mov      rdx, ADDR
       mov      qword ptr [rbp-0x200], rdx
       lea      rdx, G_M000_IG06
       mov      qword ptr [rbp-0x1E8], rdx
       lea      rdx, bword ptr [rbp-0x210]
       mov      qword ptr [rax+0x10], rdx
       mov      byte  ptr [rax+0x0C], 0
 
G_M000_IG04:                ;; offset=0x00DA
       mov      rdx, ADDR
 
G_M000_IG05:                ;; offset=0x00E4
       call     rdx ; Armonik.Ffi.Harness.Abi:ak_enc_reset(long)
 
G_M000_IG06:                ;; offset=0x00E6
       mov      rbx, qword ptr [rbp-0x198]
       mov      byte  ptr [rbx+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG07
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG07:                ;; offset=0x0100
       mov      rdi, bword ptr [rbp-0x208]
       mov      qword ptr [rbx+0x10], rdi
       mov      r14, gword ptr [rbp-0x1A0]
       mov      r12, gword ptr [r14+0x08]
       xor      edi, edi
       mov      dword ptr [r12+0x38], edi
       mov      rdi, gword ptr [r12+0x08]
       cmp      dword ptr [rdi+0x10], 0
       jbe      G_M000_IG72
       mov      rdi, gword ptr [rdi+0x08]
       cmp      dword ptr [rdi+0x08], 0
       jbe      G_M000_IG95
       mov      rdi, qword ptr [rdi+0x10]
       mov      qword ptr [r12+0x20], rdi
       mov      rdi, gword ptr [r12+0x10]
       cmp      dword ptr [rdi+0x10], 0
       jbe      G_M000_IG72
       mov      rdi, gword ptr [rdi+0x08]
       cmp      dword ptr [rdi+0x08], 0
       jbe      G_M000_IG95
       mov      edi, dword ptr [rdi+0x10]
       mov      dword ptr [r12+0x3C], edi
       xor      edi, edi
       mov      dword ptr [r12+0x40], edi
 
G_M000_IG08:                ;; offset=0x016F
       xor      eax, eax
 
G_M000_IG09:                ;; offset=0x0171
       mov      rdi, gword ptr [r12+0x18]
       cmp      eax, dword ptr [rdi+0x10]
       jl       G_M000_IG80
 
G_M000_IG10:                ;; offset=0x017F
       inc      dword ptr [rdi+0x14]
       xor      eax, eax
       mov      dword ptr [rdi+0x10], eax
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax+0x10], 3
       jl       G_M000_IG81
       mov      rdi, qword ptr [rax+0x18]
       mov      rdi, qword ptr [rdi+0x18]
       test     rdi, rdi
       je       G_M000_IG81
       mov      rax, bword ptr [rdi]
       add      rax, 16
       mov      rcx, gword ptr [rax+0x08]
       test     rcx, rcx
       jne      G_M000_IG82
 
G_M000_IG11:                ;; offset=0x01CC
       cmp      dword ptr [(reloc ADDR)], 7
       je       G_M000_IG83
 
G_M000_IG12:                ;; offset=0x01D9
       mov      edi, dword ptr [(reloc ADDR)]
       mov      r12d, 1
       mov      eax, 2
       xor      ecx, ecx
       cmp      edi, 7
       cmovne   eax, ecx
       cmp      edi, 6
       cmovne   r12d, eax
 
G_M000_IG13:                ;; offset=0x01F9
       xor      eax, eax
 
G_M000_IG14:                ;; offset=0x01FB
       test     r12d, r12d
       je       SHORT G_M000_IG17
 
G_M000_IG15:                ;; offset=0x0200
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax], 2
       jl       G_M000_IG84
       mov      rdx, qword ptr [rax+0x08]
       mov      rax, qword ptr [rdx+0x10]
       test     rax, rax
       je       G_M000_IG84
 
G_M000_IG16:                ;; offset=0x0230
       mov      rcx, rax
       mov      rax, qword ptr [rcx+0x128]
       mov      qword ptr [rbp-0x30], rax
       mov      rcx, qword ptr [rcx+0x130]
       mov      qword ptr [rbp-0x38], rcx
       mov      rax, qword ptr [rbp-0x30]
       mov      rcx, qword ptr [rbp-0x38]
 
G_M000_IG17:                ;; offset=0x0251
       mov      rdx, qword ptr [r14+0x28]
       mov      edi, dword ptr [r14+0x40]
       mov      dword ptr [rdx], edi
       mov      rdx, qword ptr [r14+0x28]
       xor      edi, edi
       test     r15b, r15b
       setne    dil
       mov      dword ptr [rdx+0x04], edi
       xor      edx, edx
       mov      qword ptr [rbp-0x40], rdx
       mov      r8, gword ptr [rbp-0x1A8]
       mov      r9, gword ptr [r8+0x08]
       mov      gword ptr [rbp-0x1D0], r9
       mov      rdx, r9
       test     rdx, rdx
       je       G_M000_IG59
 
G_M000_IG18:                ;; offset=0x028F
       mov      r10, gword ptr [rdx+0x18]
 
G_M000_IG19:                ;; offset=0x0293
       test     r10, r10
       je       G_M000_IG85
 
G_M000_IG20:                ;; offset=0x029C
       mov      gword ptr [rbp-0x1C0], r10
       test     r15b, r15b
       jne      G_M000_IG60
 
G_M000_IG21:                ;; offset=0x02AC
       vxorps   ymm0, ymm0, ymm0
       vmovdqu32 zmmword ptr [rbp-0x128], zmm0
       vmovdqu  ymmword ptr [rbp-0xF0], ymm0
       mov      r15, gword ptr [r14+0x08]
       vxorps   ymm0, ymm0, ymm0
       vmovdqu32 zmmword ptr [rbp-0x128], zmm0
       vmovdqu  ymmword ptr [rbp-0xF0], ymm0
       test     r9, r9
       je       SHORT G_M000_IG25
 
G_M000_IG22:                ;; offset=0x02E1
       mov      gword ptr [rbp-0x1C8], r9
       vxorps   ymm0, ymm0, ymm0
       vmovdqu32 zmmword ptr [rbp-0x128], zmm0
       vmovdqu  xmmword ptr [rbp-0xE8], xmm0
       mov      rdx, gword ptr [r9+0x08]
       cmp      byte  ptr [r15], r15b
       test     rdx, rdx
       je       SHORT G_M000_IG24
 
G_M000_IG23:                ;; offset=0x030A
       cmp      dword ptr [rdx+0x08], 0
       jne      SHORT G_M000_IG26
 
G_M000_IG24:                ;; offset=0x0310
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbp-0x168], xmm0
       vmovdqu  xmmword ptr [rbp-0x160], xmm0
       mov      gword ptr [rbp-0x1A8], r8
       mov      qword ptr [rbp-0x30], rax
       mov      qword ptr [rbp-0x38], rcx
       jmp      SHORT G_M000_IG27
 
G_M000_IG25:                ;; offset=0x0335
       mov      gword ptr [rbp-0x1A8], r8
       mov      qword ptr [rbp-0x30], rax
       mov      qword ptr [rbp-0x38], rcx
       jmp      G_M000_IG33
 
G_M000_IG26:                ;; offset=0x0349
       mov      gword ptr [rbp-0x1A8], r8
       mov      qword ptr [rbp-0x30], rax
       mov      qword ptr [rbp-0x38], rcx
       lea      rsi, [rbp-0x168]
       mov      rdi, r15
       call     [Armonik.Ffi.Harness.Stage:StrPresent(System.String):Armonik.Ffi.Harness.ak_str:this]
 
G_M000_IG27:                ;; offset=0x0368
       vmovdqu  xmm0, xmmword ptr [rbp-0x168]
       vmovdqu  xmmword ptr [rbp-0x128], xmm0
       mov      rsi, qword ptr [rbp-0x158]
       mov      qword ptr [rbp-0x118], rsi
       mov      rax, gword ptr [rbp-0x1C8]
       mov      rdx, gword ptr [rax+0x10]
       test     rdx, rdx
       je       SHORT G_M000_IG29
 
G_M000_IG28:                ;; offset=0x0396
       cmp      dword ptr [rdx+0x08], 0
       jne      SHORT G_M000_IG30
 
G_M000_IG29:                ;; offset=0x039C
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbp-0x180], xmm0
       vmovdqu  xmmword ptr [rbp-0x178], xmm0
       jmp      SHORT G_M000_IG31
 
G_M000_IG30:                ;; offset=0x03B2
       lea      rsi, [rbp-0x180]
       mov      rdi, r15
       call     [Armonik.Ffi.Harness.Stage:StrPresent(System.String):Armonik.Ffi.Harness.ak_str:this]
 
G_M000_IG31:                ;; offset=0x03C2
       vmovdqu  xmm0, xmmword ptr [rbp-0x180]
       vmovdqu  xmmword ptr [rbp-0x110], xmm0
       mov      rdi, qword ptr [rbp-0x170]
       mov      qword ptr [rbp-0x100], rdi
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbp-0x150], xmm0
       vmovdqu  xmmword ptr [rbp-0x148], xmm0
       mov      qword ptr [rbp-0x150], 1
       lea      r15, bword ptr [rbp-0x128]
       lea      rax, bword ptr [rbp-0x150]
       mov      rdi, gword ptr [rbp-0x1C8]
       mov      rdi, gword ptr [rdi+0x18]
       test     rdi, rdi
       je       G_M000_IG86
       mov      ecx, dword ptr [rdi+0x08]
 
G_M000_IG32:                ;; offset=0x0424
       movsxd   rdi, ecx
       mov      qword ptr [rax+0x08], rdi
       xor      edi, edi
       mov      qword ptr [rbp-0x140], rdi
       vmovdqu  xmm0, xmmword ptr [rbp-0x150]
       vmovdqu  xmmword ptr [r15+0x30], xmm0
       mov      rdi, qword ptr [rbp-0x140]
       mov      qword ptr [r15+0x40], rdi
       mov      edi, dword ptr [rbp-0xD8]
       or       edi, 1
       mov      dword ptr [rbp-0xD8], edi
 
G_M000_IG33:                ;; offset=0x045C
       test     r13b, r13b
       je       G_M000_IG91
       inc      qword ptr [(reloc ADDR)]
       test     r12d, r12d
       je       G_M000_IG37
 
G_M000_IG34:                ;; offset=0x0475
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax], 2
       jl       G_M000_IG87
       mov      rdi, qword ptr [rax+0x08]
       mov      rcx, qword ptr [rdi+0x10]
       test     rcx, rcx
       je       G_M000_IG87
       mov      r15, rcx
       mov      r13, qword ptr [rbp-0x30]
       cmp      qword ptr [r15+0x128], r13
       je       G_M000_IG88
 
G_M000_IG35:                ;; offset=0x04B9
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax+0x10], 2
       jl       G_M000_IG89
       mov      rdi, qword ptr [rax+0x18]
       mov      rdi, qword ptr [rdi+0x10]
       test     rdi, rdi
       je       G_M000_IG89
       mov      rax, bword ptr [rdi]
       add      rax, 16
 
G_M000_IG36:                ;; offset=0x04F1
       lea      rdi, bword ptr [rax+0x38]
       mov      rsi, gword ptr [rbp-0x1A8]
       call     CORINFO_HELP_ASSIGN_REF
       mov      dword ptr [r15+0x150], r12d
       mov      qword ptr [rbp-0x30], r13
 
G_M000_IG37:                ;; offset=0x050C
       test     r12d, r12d
       jne      G_M000_IG46
 
G_M000_IG38:                ;; offset=0x0515
       mov      r15, gword ptr [rbp-0x1C0]
       mov      gword ptr [rbp-0xD0], r15
       test     r15, r15
       je       SHORT G_M000_IG40
 
G_M000_IG39:                ;; offset=0x0528
       mov      rdi, gword ptr [rbp-0xD0]
       cmp      dword ptr [rdi+0x08], 0
       jne      G_M000_IG90
 
G_M000_IG40:                ;; offset=0x0539
       xor      r8d, r8d
 
G_M000_IG41:                ;; offset=0x053C
       mov      rdi, qword ptr [r14+0x28]
       mov      gword ptr [rbp-0x1A0], r14
       mov      rsi, qword ptr [r14+0x18]
       mov      r9d, dword ptr [r15+0x08]
       lea      rcx, [rbp-0x128]
       lea      rdx, [rbp-0x40]
       mov      rax, ADDR
       mov      qword ptr [rbp-0x200], rax
       lea      rax, G_M000_IG44
       mov      qword ptr [rbp-0x1E8], rax
       lea      rax, bword ptr [rbp-0x210]
       mov      qword ptr [rbx+0x10], rax
       mov      byte  ptr [rbx+0x0C], 0
 
G_M000_IG42:                ;; offset=0x0588
       mov      rax, ADDR
 
G_M000_IG43:                ;; offset=0x0592
       call     rax ; Armonik.Ffi.Harness.Abi:ak_encode_UploadResultDataMessage(ulong,long,ulong,ulong,ulong,ulong):long
 
G_M000_IG44:                ;; offset=0x0594
       mov      byte  ptr [rbx+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG45
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG45:                ;; offset=0x05A7
       mov      rdi, bword ptr [rbp-0x208]
       mov      qword ptr [rbx+0x10], rdi
       mov      r14, rax
       jmp      G_M000_IG67
 
G_M000_IG46:                ;; offset=0x05BA
       mov      r15, gword ptr [rbp-0x1C0]
       cmp      r12d, 1
       jne      G_M000_IG58
 
G_M000_IG47:                ;; offset=0x05CB
       mov      rdi, qword ptr [r14+0x28]
       mov      rsi, qword ptr [r14+0x18]
       lea      rdx, [rbp-0x40]
       lea      rcx, [rbp-0x128]
       mov      r8, gword ptr [rbp-0x1A8]
       mov      r9, r15
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinR_e(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
 
G_M000_IG48:                ;; offset=0x05EE
       mov      r15, rax
 
G_M000_IG49:                ;; offset=0x05F1
       mov      rbx, gword ptr [r14+0x08]
       cmp      byte  ptr [rbx], bl
       xor      eax, eax
 
G_M000_IG50:                ;; offset=0x05F9
       mov      rdi, gword ptr [rbx+0x18]
       cmp      eax, dword ptr [rdi+0x10]
       jl       G_M000_IG71
 
G_M000_IG51:                ;; offset=0x0606
       inc      dword ptr [rdi+0x14]
       xor      eax, eax
       mov      dword ptr [rdi+0x10], eax
       test     r12d, r12d
       je       G_M000_IG55
 
G_M000_IG52:                ;; offset=0x0617
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax], 2
       jl       G_M000_IG57
       mov      rdi, qword ptr [rax+0x08]
       mov      rax, qword ptr [rdi+0x10]
       test     rax, rax
       je       G_M000_IG57
 
G_M000_IG53:                ;; offset=0x0647
       mov      rbx, rax
       xor      edi, edi
       mov      dword ptr [rbx+0x150], edi
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax+0x10], 2
       jl       G_M000_IG74
       mov      rdi, qword ptr [rax+0x18]
       mov      rdi, qword ptr [rdi+0x10]
       test     rdi, rdi
       je       G_M000_IG74
       mov      rax, bword ptr [rdi]
       add      rax, 16
 
G_M000_IG54:                ;; offset=0x068A
       xor      rdi, rdi
       mov      gword ptr [rax+0x38], rdi
       mov      rdi, qword ptr [rbx+0x128]
       sub      rdi, qword ptr [rbp-0x30]
       mov      rax, qword ptr [rbx+0x130]
       sub      rax, qword ptr [rbp-0x38]
       cmp      rdi, rax
       jne      G_M000_IG75
 
G_M000_IG55:                ;; offset=0x06AF
       test     r15, r15
       jl       G_M000_IG76
       cmp      byte  ptr [r14+0x4C], 0
       jne      G_M000_IG91
 
G_M000_IG56:                ;; offset=0x06C3
       jmp      G_M000_IG77
 
G_M000_IG57:                ;; offset=0x06C8
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG53
 
G_M000_IG58:                ;; offset=0x06D7
       mov      rdi, qword ptr [r14+0x28]
       mov      rsi, qword ptr [r14+0x18]
       lea      rdx, [rbp-0x40]
       lea      rcx, [rbp-0x128]
       mov      r8, gword ptr [rbp-0x1A8]
       mov      r9, r15
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinH_e(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       jmp      G_M000_IG48
 
G_M000_IG59:                ;; offset=0x06FF
       xor      r10, r10
       jmp      G_M000_IG19
 
G_M000_IG60:                ;; offset=0x0707
       mov      qword ptr [rbp-0x30], rax
       mov      qword ptr [rbp-0x38], rcx
       vxorps   ymm2, ymm2, ymm2
       vmovdqu32 zmmword ptr [rbp-0xC8], zmm2
       vmovdqu32 zmmword ptr [rbp-0x90], zmm2
       mov      rdx, gword ptr [r14+0x08]
       lea      rdi, [rbp-0xC8]
       mov      gword ptr [rbp-0x1A8], r8
       mov      rsi, r8
       call     [Armonik.Ffi.Harness.G:U_UploadResultDataMessage(byref,Armonik.Ffi.Facade.UploadResultDataMessage,Armonik.Ffi.Harness.Stage)]
       test     r13b, r13b
       je       G_M000_IG91
       inc      qword ptr [(reloc ADDR)]
       test     r12d, r12d
       jne      G_M000_IG92
 
G_M000_IG61:                ;; offset=0x075B
       test     r12d, r12d
       jne      G_M000_IG68
       mov      r15, gword ptr [rbp-0x1C0]
       mov      gword ptr [rbp-0xD0], r15
       test     r15, r15
       jne      G_M000_IG94
 
G_M000_IG62:                ;; offset=0x077B
       xor      r8d, r8d
 
G_M000_IG63:                ;; offset=0x077E
       mov      rdi, qword ptr [r14+0x28]
       mov      gword ptr [rbp-0x1A0], r14
       mov      rsi, qword ptr [r14+0x18]
       mov      r9d, dword ptr [r15+0x08]
       lea      rcx, [rbp-0xC8]
       lea      rdx, [rbp-0x40]
       mov      rax, ADDR
       mov      qword ptr [rbp-0x200], rax
       lea      rax, G_M000_IG65
       mov      qword ptr [rbp-0x1E8], rax
       lea      rax, bword ptr [rbp-0x210]
       mov      qword ptr [rbx+0x10], rax
       mov      byte  ptr [rbx+0x0C], 0
 
G_M000_IG64:                ;; offset=0x07CA
       call     [Armonik.Ffi.Harness.Abi:ak_uencode_UploadResultDataMessage(ulong,long,ulong,ulong,ulong,ulong):long]
 
G_M000_IG65:                ;; offset=0x07D0
       mov      byte  ptr [rbx+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG66
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG66:                ;; offset=0x07E3
       mov      rsi, bword ptr [rbp-0x208]
       mov      qword ptr [rbx+0x10], rsi
       mov      r14, rax
 
G_M000_IG67:                ;; offset=0x07F1
       xor      rsi, rsi
       mov      gword ptr [rbp-0xD0], rsi
       mov      r15, r14
       mov      r14, gword ptr [rbp-0x1A0]
       jmp      G_M000_IG49
 
G_M000_IG68:                ;; offset=0x0809
       mov      r15, gword ptr [rbp-0x1C0]
       cmp      r12d, 1
       je       SHORT G_M000_IG69
       mov      rdi, qword ptr [r14+0x28]
       mov      rsi, qword ptr [r14+0x18]
       lea      rdx, [rbp-0x40]
       lea      rcx, [rbp-0xC8]
       mov      r8, gword ptr [rbp-0x1A8]
       mov      r9, r15
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinH_u(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       mov      r15, rax
       jmp      SHORT G_M000_IG70
 
G_M000_IG69:                ;; offset=0x083E
       mov      rdi, qword ptr [r14+0x28]
       mov      rsi, qword ptr [r14+0x18]
       lea      rdx, [rbp-0x40]
       lea      rcx, [rbp-0xC8]
       mov      r8, gword ptr [rbp-0x1A8]
       mov      r9, r15
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinR_u(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       mov      r15, rax
 
G_M000_IG70:                ;; offset=0x0864
       jmp      G_M000_IG49
 
G_M000_IG71:                ;; offset=0x0869
       mov      rcx, rdi
       cmp      eax, dword ptr [rcx+0x10]
       jb       SHORT G_M000_IG73
 
G_M000_IG72:                ;; offset=0x0871
       call     [System.ThrowHelper:ThrowArgumentOutOfRange_IndexMustBeLessException()]
       int3     
 
G_M000_IG73:                ;; offset=0x0878
       mov      rdi, rcx
       mov      r13, qword ptr [rbp-0x30]
       mov      rdi, gword ptr [rdi+0x08]
       cmp      eax, dword ptr [rdi+0x08]
       jae      G_M000_IG95
       mov      dword ptr [rbp-0x184], eax
       mov      edx, eax
       mov      rdi, qword ptr [rdi+8*rdx+0x10]
       mov      qword ptr [rbp-0x190], rdi
       lea      rdi, [rbp-0x190]
       call     [System.Runtime.InteropServices.GCHandle:Free():this]
       mov      eax, dword ptr [rbp-0x184]
       inc      eax
       mov      qword ptr [rbp-0x30], r13
       jmp      G_M000_IG50
 
G_M000_IG74:                ;; offset=0x08BE
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG54
 
G_M000_IG75:                ;; offset=0x08CD
       mov      rax, -1
       test     r15, r15
       cmovge   r15, rax
       jmp      G_M000_IG55
 
G_M000_IG76:                ;; offset=0x08E3
       mov      eax, r15d
       jmp      SHORT G_M000_IG79
 
G_M000_IG77:                ;; offset=0x08E8
       mov      rdi, qword ptr [r14+0x18]
       lea      rsi, [rbp-0x48]
       lea      rdx, [rbp-0x50]
       call     Armonik.Ffi.Harness.Abi:ak_enc_take(long,ulong,ulong):int
       test     eax, eax
       je       SHORT G_M000_IG78
       jmp      SHORT G_M000_IG79
 
G_M000_IG78:                ;; offset=0x08FF
       mov      rax, qword ptr [rbp-0x48]
       mov      r12, bword ptr [rbp-0x1B0]
       mov      qword ptr [r12], rax
       mov      eax, dword ptr [rbp-0x50]
       mov      rbx, bword ptr [rbp-0x1B8]
       mov      dword ptr [rbx], eax
       jmp      G_M000_IG91
 
G_M000_IG79:                ;; offset=0x091F
       add      rsp, 504
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG80:                ;; offset=0x0931
       cmp      eax, dword ptr [rdi+0x10]
       jae      G_M000_IG72
       mov      rdi, gword ptr [rdi+0x08]
       cmp      eax, dword ptr [rdi+0x08]
       jae      G_M000_IG95
       mov      dword ptr [rbp-0x12C], eax
       mov      ecx, eax
       mov      rdi, qword ptr [rdi+8*rcx+0x10]
       mov      qword ptr [rbp-0x138], rdi
       lea      rdi, [rbp-0x138]
       call     [System.Runtime.InteropServices.GCHandle:Free():this]
       mov      eax, dword ptr [rbp-0x12C]
       inc      eax
       jmp      G_M000_IG09
 
G_M000_IG81:                ;; offset=0x0975
       mov      edi, 3
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      rcx, gword ptr [rax+0x08]
       test     rcx, rcx
       je       G_M000_IG11
 
G_M000_IG82:                ;; offset=0x098C
       inc      dword ptr [rcx+0x14]
       mov      edx, dword ptr [rcx+0x10]
       xor      edi, edi
       mov      dword ptr [rcx+0x10], edi
       test     edx, edx
       jle      G_M000_IG11
       mov      rdi, gword ptr [rcx+0x08]
       xor      esi, esi
       call     [System.Array:Clear(System.Array,int,int)]
       cmp      dword ptr [(reloc ADDR)], 7
       jne      G_M000_IG12
 
G_M000_IG83:                ;; offset=0x09B8
       xor      edi, edi
       call     [Armonik.Ffi.Harness.Stage:ReleaseChunk(int)]
       jmp      G_M000_IG12
 
G_M000_IG84:                ;; offset=0x09C5
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG16
 
G_M000_IG85:                ;; offset=0x09D4
       mov      gword ptr [rbp-0x1A8], r8
       mov      qword ptr [rbp-0x30], rax
       mov      qword ptr [rbp-0x38], rcx
       mov      rdi, ADDR
       mov      esi, 11
       call     CORINFO_HELP_CLASSINIT_SHARED_DYNAMICCLASS
       mov      rdi, ADDR
       mov      r10, gword ptr [rdi]
       mov      rax, qword ptr [rbp-0x30]
       mov      rcx, qword ptr [rbp-0x38]
       mov      r8, gword ptr [rbp-0x1A8]
       mov      r9, gword ptr [rbp-0x1D0]
       jmp      G_M000_IG20
 
G_M000_IG86:                ;; offset=0x0A1F
       xor      ecx, ecx
       jmp      G_M000_IG32
 
G_M000_IG87:                ;; offset=0x0A26
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      rcx, rax
       mov      r15, rcx
       mov      r13, qword ptr [rbp-0x30]
       cmp      qword ptr [r15+0x128], r13
       jne      G_M000_IG35
 
G_M000_IG88:                ;; offset=0x0A47
       xor      r12d, r12d
       mov      qword ptr [rbp-0x30], r13
       jmp      G_M000_IG37
 
G_M000_IG89:                ;; offset=0x0A53
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG36
 
G_M000_IG90:                ;; offset=0x0A62
       mov      rax, gword ptr [rbp-0xD0]
       cmp      dword ptr [rax+0x08], 0
       jbe      G_M000_IG95
       mov      r8, gword ptr [rbp-0xD0]
       add      r8, 16
       jmp      G_M000_IG41
 
G_M000_IG91:                ;; offset=0x0A83
       xor      eax, eax
       jmp      G_M000_IG79
 
G_M000_IG92:                ;; offset=0x0A8A
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      r15, rax
       mov      r13, qword ptr [rbp-0x30]
       cmp      qword ptr [r15+0x128], r13
       jne      SHORT G_M000_IG93
       xor      r12d, r12d
       mov      qword ptr [rbp-0x30], r13
       jmp      G_M000_IG61
 
G_M000_IG93:                ;; offset=0x0AB0
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       lea      rdi, bword ptr [rax+0x38]
       mov      rsi, gword ptr [rbp-0x1A8]
       call     CORINFO_HELP_ASSIGN_REF
       mov      dword ptr [r15+0x150], r12d
       mov      qword ptr [rbp-0x30], r13
       jmp      G_M000_IG61
 
G_M000_IG94:                ;; offset=0x0ADA
       mov      rdi, gword ptr [rbp-0xD0]
       cmp      dword ptr [rdi+0x08], 0
       je       G_M000_IG62
       mov      rdi, gword ptr [rbp-0xD0]
       cmp      dword ptr [rdi+0x08], 0
       jbe      SHORT G_M000_IG95
       mov      r8, gword ptr [rbp-0xD0]
       add      r8, 16
       jmp      G_M000_IG63
 
G_M000_IG95:                ;; offset=0x0B08
       call     CORINFO_HELP_RNGCHKFAIL
       int3     
 
; Total bytes of code 2830

