; Assembly listing for method Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:Go(Armonik.Ffi.Facade.UploadResultDataMessage,bool,bool,byref,byref):int:this (Tier1)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1 code
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; partially interruptible
; with Dynamic PGO: edge weights are invalid, and fgCalledCount is 34232
; 12 inlinees with PGO data; 7 single block inlinees; 3 inlinees without PGO data

G_M000_IG01:                ;; offset=0x0000
       push     rbp
       push     r15
       push     r14
       push     r13
       push     r12
       push     rbx
       sub      rsp, 488
       vzeroupper 
       lea      rbp, [rsp+0x210]
       xor      ebx, ebx
       mov      qword ptr [rbp-0x188], rbx
       vxorps   xmm8, xmm8, xmm8
       vmovdqa  xmmword ptr [rbp-0x180], xmm8
       vmovdqa  xmmword ptr [rbp-0x170], xmm8
       mov      rbx, -288
       vmovdqa  xmmword ptr [rbp+rbx-0x40], xmm8
       vmovdqa  xmmword ptr [rbp+rbx-0x30], xmm8
       vmovdqa  xmmword ptr [rbp+rbx-0x20], xmm8
       add      rbx, 48
       jne      SHORT  -5 instr
       mov      qword ptr [rbp-0x40], rbx
       mov      gword ptr [rbp-0x1A0], rsi
       mov      bword ptr [rbp-0x1B0], r9
       mov      rbx, rdi
       mov      r15d, edx
       mov      r13d, ecx
       mov      r12, r8
 
G_M000_IG02:                ;; offset=0x007A
       lea      rdi, [rbp-0x208]
       mov      rsi, r10
       call     CORINFO_HELP_INIT_PINVOKE_FRAME
       mov      qword ptr [rbp-0x190], rax
       mov      rdi, rsp
       mov      qword ptr [rbp-0x1E8], rdi
       mov      rdi, rbp
       mov      qword ptr [rbp-0x1D8], rdi
       xor      edi, edi
       mov      bword ptr [rbp-0x1A8], r12
       mov      qword ptr [r12], rdi
 
G_M000_IG03:                ;; offset=0x00B1
       mov      rcx, bword ptr [rbp-0x1B0]
       mov      dword ptr [rcx], edi
       mov      gword ptr [rbp-0x198], rbx
       mov      rdi, qword ptr [rbx+0x18]
       mov      rdx, ADDR
       mov      qword ptr [rbp-0x1F8], rdx
       lea      rdx, G_M000_IG06
       mov      qword ptr [rbp-0x1E0], rdx
       lea      rdx, bword ptr [rbp-0x208]
       mov      qword ptr [rax+0x10], rdx
       mov      byte  ptr [rax+0x0C], 0
 
G_M000_IG04:                ;; offset=0x00F3
       mov      rdx, ADDR
 
G_M000_IG05:                ;; offset=0x00FD
       call     rdx ; Armonik.Ffi.Harness.Abi:ak_enc_reset(long)
 
G_M000_IG06:                ;; offset=0x00FF
       mov      rbx, qword ptr [rbp-0x190]
       mov      byte  ptr [rbx+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG07
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG07:                ;; offset=0x0119
       mov      rdi, bword ptr [rbp-0x200]
       mov      qword ptr [rbx+0x10], rdi
       mov      r14, gword ptr [rbp-0x198]
       mov      r12, gword ptr [r14+0x08]
       xor      edi, edi
       mov      dword ptr [r12+0x38], edi
       mov      rdi, gword ptr [r12+0x08]
       cmp      dword ptr [rdi+0x10], 0
       jbe      G_M000_IG87
       mov      rdi, gword ptr [rdi+0x08]
       cmp      dword ptr [rdi+0x08], 0
       jbe      G_M000_IG95
       mov      rdi, qword ptr [rdi+0x10]
       mov      qword ptr [r12+0x20], rdi
       mov      rdi, gword ptr [r12+0x10]
       cmp      dword ptr [rdi+0x10], 0
       jbe      G_M000_IG87
       mov      rdi, gword ptr [rdi+0x08]
       cmp      dword ptr [rdi+0x08], 0
       jbe      G_M000_IG95
       mov      edi, dword ptr [rdi+0x10]
       mov      dword ptr [r12+0x3C], edi
       xor      edi, edi
       mov      dword ptr [r12+0x40], edi
 
G_M000_IG08:                ;; offset=0x0188
       xor      eax, eax
 
G_M000_IG09:                ;; offset=0x018A
       mov      rdi, gword ptr [r12+0x18]
       cmp      eax, dword ptr [rdi+0x10]
       jl       G_M000_IG71
 
G_M000_IG10:                ;; offset=0x0198
       inc      dword ptr [rdi+0x14]
       xor      eax, eax
       mov      dword ptr [rdi+0x10], eax
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax+0x10], 3
       jl       G_M000_IG72
       mov      rdi, qword ptr [rax+0x18]
       mov      rdi, qword ptr [rdi+0x18]
       test     rdi, rdi
       je       G_M000_IG72
       mov      rax, bword ptr [rdi]
       add      rax, 16
       mov      rcx, gword ptr [rax+0x08]
       test     rcx, rcx
       jne      G_M000_IG73
 
G_M000_IG11:                ;; offset=0x01E5
       cmp      dword ptr [(reloc ADDR)], 7
       je       G_M000_IG74
 
G_M000_IG12:                ;; offset=0x01F2
       mov      edi, dword ptr [(reloc ADDR)]
       mov      r12d, 1
       mov      eax, 2
       xor      ecx, ecx
       cmp      edi, 7
       cmovne   eax, ecx
       cmp      edi, 6
       cmovne   r12d, eax
 
G_M000_IG13:                ;; offset=0x0212
       xor      eax, eax
 
G_M000_IG14:                ;; offset=0x0214
       test     r12d, r12d
       jne      G_M000_IG65
 
G_M000_IG15:                ;; offset=0x021D
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
       mov      r8, gword ptr [rbp-0x1A0]
       mov      r9, gword ptr [r8+0x08]
       mov      gword ptr [rbp-0x1C8], r9
       mov      rdx, r9
       test     rdx, rdx
       je       G_M000_IG64
 
G_M000_IG16:                ;; offset=0x025B
       mov      r10, gword ptr [rdx+0x18]
 
G_M000_IG17:                ;; offset=0x025F
       test     r10, r10
       je       G_M000_IG75
 
G_M000_IG18:                ;; offset=0x0268
       mov      gword ptr [rbp-0x1B8], r10
       test     r15b, r15b
       jne      G_M000_IG56
 
G_M000_IG19:                ;; offset=0x0278
       vxorps   ymm0, ymm0, ymm0
       vmovdqu32 zmmword ptr [rbp-0x128], zmm0
       vmovdqu  ymmword ptr [rbp-0xF0], ymm0
       mov      r15, gword ptr [r14+0x08]
       vxorps   ymm0, ymm0, ymm0
       vmovdqu32 zmmword ptr [rbp-0x128], zmm0
       vmovdqu  ymmword ptr [rbp-0xF0], ymm0
       test     r9, r9
       je       G_M000_IG28
 
G_M000_IG20:                ;; offset=0x02B1
       mov      gword ptr [rbp-0x1C0], r9
       vxorps   ymm0, ymm0, ymm0
       vmovdqu32 zmmword ptr [rbp-0x128], zmm0
       vmovdqu  xmmword ptr [rbp-0xE8], xmm0
       mov      rdx, gword ptr [r9+0x08]
       cmp      byte  ptr [r15], r15b
       test     rdx, rdx
       je       SHORT G_M000_IG22
 
G_M000_IG21:                ;; offset=0x02DA
       cmp      dword ptr [rdx+0x08], 0
       jne      G_M000_IG41
 
G_M000_IG22:                ;; offset=0x02E4
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbp-0x168], xmm0
       vmovdqu  xmmword ptr [rbp-0x160], xmm0
 
G_M000_IG23:                ;; offset=0x02F8
       vmovdqu  xmm0, xmmword ptr [rbp-0x168]
       vmovdqu  xmmword ptr [rbp-0x128], xmm0
       mov      rsi, qword ptr [rbp-0x158]
       mov      qword ptr [rbp-0x118], rsi
       mov      r9, gword ptr [rbp-0x1C0]
       mov      rdx, gword ptr [r9+0x10]
       test     rdx, rdx
       je       SHORT G_M000_IG25
 
G_M000_IG24:                ;; offset=0x0326
       cmp      dword ptr [rdx+0x08], 0
       jne      G_M000_IG45
 
G_M000_IG25:                ;; offset=0x0330
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbp-0x180], xmm0
       vmovdqu  xmmword ptr [rbp-0x178], xmm0
 
G_M000_IG26:                ;; offset=0x0344
       vmovdqu  xmm0, xmmword ptr [rbp-0x180]
       vmovdqu  xmmword ptr [rbp-0x110], xmm0
       mov      rdi, qword ptr [rbp-0x170]
       mov      qword ptr [rbp-0x100], rdi
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbp-0x150], xmm0
       vmovdqu  xmmword ptr [rbp-0x148], xmm0
       mov      qword ptr [rbp-0x150], 1
       lea      r15, bword ptr [rbp-0x128]
       lea      rdx, bword ptr [rbp-0x150]
       mov      r9, gword ptr [rbp-0x1C0]
       mov      rdi, gword ptr [r9+0x18]
       test     rdi, rdi
       je       G_M000_IG51
       mov      esi, dword ptr [rdi+0x08]
 
G_M000_IG27:                ;; offset=0x03A6
       movsxd   rdi, esi
       mov      qword ptr [rdx+0x08], rdi
       xor      edi, edi
       mov      qword ptr [rbp-0x140], rdi
       vmovdqu  xmm0, xmmword ptr [rbp-0x150]
       vmovdqu  xmmword ptr [r15+0x30], xmm0
       mov      rdi, qword ptr [rbp-0x140]
       mov      qword ptr [r15+0x40], rdi
       mov      edi, dword ptr [rbp-0xD8]
       or       edi, 1
       mov      dword ptr [rbp-0xD8], edi
 
G_M000_IG28:                ;; offset=0x03DE
       test     r13b, r13b
       je       G_M000_IG78
       mov      r15, ADDR
       inc      qword ptr [r15]
       test     r12d, r12d
       jne      G_M000_IG38
 
G_M000_IG29:                ;; offset=0x03FD
       test     r12d, r12d
       jne      G_M000_IG55
 
G_M000_IG30:                ;; offset=0x0406
       mov      r11, gword ptr [rbp-0x1B8]
       mov      gword ptr [rbp-0xD0], r11
       test     r11, r11
       je       SHORT G_M000_IG32
 
G_M000_IG31:                ;; offset=0x0419
       mov      rdi, gword ptr [rbp-0xD0]
       cmp      dword ptr [rdi+0x08], 0
       jne      G_M000_IG54
 
G_M000_IG32:                ;; offset=0x042A
       xor      r8d, r8d
       mov      qword ptr [rbp-0x30], rax
       mov      qword ptr [rbp-0x38], rcx
 
G_M000_IG33:                ;; offset=0x0435
       mov      rdi, qword ptr [r14+0x28]
       mov      gword ptr [rbp-0x198], r14
       mov      rsi, qword ptr [r14+0x18]
       mov      r9d, dword ptr [r11+0x08]
       lea      rcx, [rbp-0x128]
       lea      rdx, [rbp-0x40]
       mov      r10, ADDR
       mov      qword ptr [rbp-0x1F8], r10
       lea      r10, G_M000_IG36
       mov      qword ptr [rbp-0x1E0], r10
       lea      r10, bword ptr [rbp-0x208]
       mov      qword ptr [rbx+0x10], r10
       mov      byte  ptr [rbx+0x0C], 0
 
G_M000_IG34:                ;; offset=0x0481
       mov      r10, ADDR
 
G_M000_IG35:                ;; offset=0x048B
       call     r10 ; Armonik.Ffi.Harness.Abi:ak_encode_UploadResultDataMessage(ulong,long,ulong,ulong,ulong,ulong):long
 
G_M000_IG36:                ;; offset=0x048E
       mov      byte  ptr [rbx+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG37
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG37:                ;; offset=0x04A1
       mov      rdi, bword ptr [rbp-0x200]
       mov      qword ptr [rbx+0x10], rdi
       mov      r14, rax
       jmp      G_M000_IG63
 
G_M000_IG38:                ;; offset=0x04B4
       mov      gword ptr [rbp-0x1A0], r8
       mov      qword ptr [rbp-0x30], rax
       mov      qword ptr [rbp-0x38], rcx
       mov      rdi, ADDR
       mov      rdx, ADDR
       call     rdx
       cmp      dword ptr [rax], 2
       jl       G_M000_IG76
       mov      rdi, qword ptr [rax+0x08]
       mov      rcx, qword ptr [rdi+0x10]
       test     rcx, rcx
       je       G_M000_IG76
 
G_M000_IG39:                ;; offset=0x04F3
       mov      r15, rcx
       mov      r13, qword ptr [rbp-0x30]
       cmp      qword ptr [r15+0x128], r13
       je       G_M000_IG52
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax+0x10], 2
       jl       G_M000_IG53
       mov      rdi, qword ptr [rax+0x18]
       mov      rdi, qword ptr [rdi+0x10]
       test     rdi, rdi
       je       G_M000_IG53
       mov      rax, bword ptr [rdi]
       add      rax, 16
 
G_M000_IG40:                ;; offset=0x053F
       lea      rdi, bword ptr [rax+0x38]
       mov      rsi, gword ptr [rbp-0x1A0]
       call     CORINFO_HELP_ASSIGN_REF
       mov      dword ptr [r15+0x150], r12d
       mov      rax, qword ptr [rbp-0x30]
       mov      rcx, qword ptr [rbp-0x38]
       mov      r8, gword ptr [rbp-0x1A0]
       jmp      G_M000_IG29
 
G_M000_IG41:                ;; offset=0x056A
       mov      gword ptr [rbp-0x1A0], r8
       mov      qword ptr [rbp-0x30], rax
       mov      qword ptr [rbp-0x38], rcx
       lea      rsi, [rbp-0x168]
       mov      rdi, r15
       call     [Armonik.Ffi.Harness.Stage:StrPresent(System.String):Armonik.Ffi.Harness.ak_str:this]
       mov      rax, qword ptr [rbp-0x30]
       mov      rcx, qword ptr [rbp-0x38]
       mov      r8, gword ptr [rbp-0x1A0]
       jmp      G_M000_IG23
 
G_M000_IG42:                ;; offset=0x059D
       mov      rdi, gword ptr [rbx+0x18]
       cmp      r15d, dword ptr [rdi+0x10]
       jl       G_M000_IG86
 
G_M000_IG43:                ;; offset=0x05AB
       inc      dword ptr [rdi+0x14]
       xor      edx, edx
       mov      dword ptr [rdi+0x10], edx
       test     r12d, r12d
       je       G_M000_IG49
 
G_M000_IG44:                ;; offset=0x05BC
       jmp      G_M000_IG67
 
G_M000_IG45:                ;; offset=0x05C1
       mov      gword ptr [rbp-0x1A0], r8
       mov      qword ptr [rbp-0x30], rax
       mov      qword ptr [rbp-0x38], rcx
       lea      rsi, [rbp-0x180]
       mov      rdi, r15
       call     [Armonik.Ffi.Harness.Stage:StrPresent(System.String):Armonik.Ffi.Harness.ak_str:this]
       mov      rax, qword ptr [rbp-0x30]
       mov      rcx, qword ptr [rbp-0x38]
       mov      r8, gword ptr [rbp-0x1A0]
       jmp      G_M000_IG26
 
G_M000_IG46:                ;; offset=0x05F4
       mov      qword ptr [rbp-0x30], rax
       mov      qword ptr [rbp-0x38], rcx
       mov      rdi, qword ptr [r14+0x28]
       mov      rsi, qword ptr [r14+0x18]
       lea      rdx, [rbp-0x40]
       lea      rcx, [rbp-0x128]
       mov      r9, r11
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinR_e(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
 
G_M000_IG47:                ;; offset=0x0618
       mov      r13, rax
       mov      gword ptr [rbp-0x198], r14
       mov      r14, r13
 
G_M000_IG48:                ;; offset=0x0625
       mov      r13, gword ptr [rbp-0x198]
       mov      rbx, gword ptr [r13+0x08]
       cmp      byte  ptr [rbx], bl
       xor      r15d, r15d
       jmp      G_M000_IG42
 
G_M000_IG49:                ;; offset=0x063A
       test     r14, r14
       jl       G_M000_IG92
       cmp      byte  ptr [r13+0x4C], 0
       jne      G_M000_IG78
 
G_M000_IG50:                ;; offset=0x064E
       mov      r14, r13
       jmp      G_M000_IG93
 
G_M000_IG51:                ;; offset=0x0656
       mov      r11, gword ptr [rbp-0x1B8]
       xor      esi, esi
       jmp      G_M000_IG27
 
G_M000_IG52:                ;; offset=0x0664
       xor      r12d, r12d
       mov      rax, qword ptr [rbp-0x30]
       mov      rcx, qword ptr [rbp-0x38]
       mov      r8, gword ptr [rbp-0x1A0]
       jmp      G_M000_IG29
 
G_M000_IG53:                ;; offset=0x067B
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG40
 
G_M000_IG54:                ;; offset=0x068A
       mov      rdi, gword ptr [rbp-0xD0]
       cmp      dword ptr [rdi+0x08], 0
       jbe      G_M000_IG95
       mov      r8, gword ptr [rbp-0xD0]
       add      r8, 16
       mov      qword ptr [rbp-0x30], rax
       mov      qword ptr [rbp-0x38], rcx
       jmp      G_M000_IG33
 
G_M000_IG55:                ;; offset=0x06B3
       mov      r11, gword ptr [rbp-0x1B8]
       cmp      r12d, 1
       jne      G_M000_IG77
       jmp      G_M000_IG46
 
G_M000_IG56:                ;; offset=0x06C9
       mov      qword ptr [rbp-0x30], rax
       mov      qword ptr [rbp-0x38], rcx
       vxorps   ymm2, ymm2, ymm2
       vmovdqu32 zmmword ptr [rbp-0xC8], zmm2
       vmovdqu32 zmmword ptr [rbp-0x90], zmm2
       mov      rdx, gword ptr [r14+0x08]
       lea      rdi, [rbp-0xC8]
       mov      gword ptr [rbp-0x1A0], r8
       mov      rsi, r8
       call     [Armonik.Ffi.Harness.G:U_UploadResultDataMessage(byref,Armonik.Ffi.Facade.UploadResultDataMessage,Armonik.Ffi.Harness.Stage)]
       test     r13b, r13b
       je       G_M000_IG78
       mov      r15, ADDR
       inc      qword ptr [r15]
       test     r12d, r12d
       jne      G_M000_IG80
 
G_M000_IG57:                ;; offset=0x0723
       test     r12d, r12d
       jne      G_M000_IG83
       mov      r15, gword ptr [rbp-0x1B8]
       mov      gword ptr [rbp-0xD0], r15
       test     r15, r15
       jne      G_M000_IG82
 
G_M000_IG58:                ;; offset=0x0743
       xor      r8d, r8d
 
G_M000_IG59:                ;; offset=0x0746
       mov      rdi, qword ptr [r14+0x28]
       mov      gword ptr [rbp-0x198], r14
       mov      rsi, qword ptr [r14+0x18]
       mov      r9d, dword ptr [r15+0x08]
       lea      rcx, [rbp-0xC8]
       lea      rdx, [rbp-0x40]
       mov      rax, ADDR
       mov      qword ptr [rbp-0x1F8], rax
       lea      rax, G_M000_IG61
       mov      qword ptr [rbp-0x1E0], rax
       lea      rax, bword ptr [rbp-0x208]
       mov      qword ptr [rbx+0x10], rax
       mov      byte  ptr [rbx+0x0C], 0
 
G_M000_IG60:                ;; offset=0x0792
       call     [Armonik.Ffi.Harness.Abi:ak_uencode_UploadResultDataMessage(ulong,long,ulong,ulong,ulong,ulong):long]
 
G_M000_IG61:                ;; offset=0x0798
       mov      byte  ptr [rbx+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG62
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG62:                ;; offset=0x07AB
       mov      rsi, bword ptr [rbp-0x200]
       mov      qword ptr [rbx+0x10], rsi
       mov      r14, rax
 
G_M000_IG63:                ;; offset=0x07B9
       xor      rsi, rsi
       mov      gword ptr [rbp-0xD0], rsi
       jmp      G_M000_IG48
 
G_M000_IG64:                ;; offset=0x07C7
       xor      r10, r10
       jmp      G_M000_IG17
 
G_M000_IG65:                ;; offset=0x07CF
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax], 2
       jl       G_M000_IG79
       mov      rdx, qword ptr [rax+0x08]
       mov      rax, qword ptr [rdx+0x10]
       test     rax, rax
       je       G_M000_IG79
 
G_M000_IG66:                ;; offset=0x07FF
       mov      rcx, rax
       mov      rax, qword ptr [rcx+0x128]
       mov      qword ptr [rbp-0x30], rax
       mov      rcx, qword ptr [rcx+0x130]
       mov      qword ptr [rbp-0x38], rcx
       mov      rax, qword ptr [rbp-0x30]
       mov      rcx, qword ptr [rbp-0x38]
       jmp      G_M000_IG15
 
G_M000_IG67:                ;; offset=0x0825
       mov      rdi, ADDR
       mov      rdx, ADDR
       call     rdx
       cmp      dword ptr [rax], 2
       jl       G_M000_IG89
       mov      rdi, qword ptr [rax+0x08]
       mov      r15, qword ptr [rdi+0x10]
       test     r15, r15
       je       G_M000_IG89
 
G_M000_IG68:                ;; offset=0x0855
       xor      edi, edi
       mov      dword ptr [r15+0x150], edi
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax+0x10], 2
       jl       G_M000_IG90
       mov      rdi, qword ptr [rax+0x18]
       mov      rdi, qword ptr [rdi+0x10]
       test     rdi, rdi
       je       G_M000_IG90
       mov      rax, bword ptr [rdi]
       add      rax, 16
 
G_M000_IG69:                ;; offset=0x0896
       xor      rdi, rdi
       mov      gword ptr [rax+0x38], rdi
       mov      rdi, qword ptr [r15+0x128]
       sub      rdi, qword ptr [rbp-0x30]
       mov      rsi, qword ptr [r15+0x130]
       sub      rsi, qword ptr [rbp-0x38]
       cmp      rdi, rsi
       jne      G_M000_IG91
       jmp      G_M000_IG49
 
G_M000_IG70:                ;; offset=0x08C0
       add      rsp, 488
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG71:                ;; offset=0x08D2
       cmp      eax, dword ptr [rdi+0x10]
       jae      G_M000_IG87
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
 
G_M000_IG72:                ;; offset=0x0916
       mov      edi, 3
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      rcx, gword ptr [rax+0x08]
       test     rcx, rcx
       je       G_M000_IG11
 
G_M000_IG73:                ;; offset=0x092D
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
 
G_M000_IG74:                ;; offset=0x0959
       xor      edi, edi
       call     [Armonik.Ffi.Harness.Stage:ReleaseChunk(int)]
       jmp      G_M000_IG12
 
G_M000_IG75:                ;; offset=0x0966
       mov      gword ptr [rbp-0x1A0], r8
       mov      qword ptr [rbp-0x30], rax
       mov      qword ptr [rbp-0x38], rcx
       mov      rdi, ADDR
       mov      esi, 11
       call     CORINFO_HELP_CLASSINIT_SHARED_DYNAMICCLASS
       mov      rdi, ADDR
       mov      r10, gword ptr [rdi]
       mov      rdi, r10
       mov      r10, rdi
       mov      rax, qword ptr [rbp-0x30]
       mov      rcx, qword ptr [rbp-0x38]
       mov      r8, gword ptr [rbp-0x1A0]
       mov      r9, gword ptr [rbp-0x1C8]
       jmp      G_M000_IG18
 
G_M000_IG76:                ;; offset=0x09B7
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      rcx, rax
       jmp      G_M000_IG39
 
G_M000_IG77:                ;; offset=0x09C9
       mov      qword ptr [rbp-0x30], rax
       mov      qword ptr [rbp-0x38], rcx
       mov      rdi, qword ptr [r14+0x28]
       mov      rsi, qword ptr [r14+0x18]
       lea      rdx, [rbp-0x40]
       lea      rcx, [rbp-0x128]
       mov      r9, r11
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinH_e(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       jmp      G_M000_IG47
 
G_M000_IG78:                ;; offset=0x09F2
       xor      eax, eax
       jmp      G_M000_IG70
 
G_M000_IG79:                ;; offset=0x09F9
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG66
 
G_M000_IG80:                ;; offset=0x0A08
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      r15, rax
       mov      r13, qword ptr [rbp-0x30]
       cmp      qword ptr [r15+0x128], r13
       jne      SHORT G_M000_IG81
       xor      r12d, r12d
       jmp      G_M000_IG57
 
G_M000_IG81:                ;; offset=0x0A2A
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       lea      rdi, bword ptr [rax+0x38]
       mov      rsi, gword ptr [rbp-0x1A0]
       call     CORINFO_HELP_ASSIGN_REF
       mov      dword ptr [r15+0x150], r12d
       jmp      G_M000_IG57
 
G_M000_IG82:                ;; offset=0x0A50
       mov      rdi, gword ptr [rbp-0xD0]
       cmp      dword ptr [rdi+0x08], 0
       je       G_M000_IG58
       mov      rdi, gword ptr [rbp-0xD0]
       cmp      dword ptr [rdi+0x08], 0
       jbe      G_M000_IG95
       mov      r8, gword ptr [rbp-0xD0]
       add      r8, 16
       jmp      G_M000_IG59
 
G_M000_IG83:                ;; offset=0x0A82
       mov      r15, gword ptr [rbp-0x1B8]
       cmp      r12d, 1
       je       SHORT G_M000_IG84
       mov      rdi, qword ptr [r14+0x28]
       mov      rsi, qword ptr [r14+0x18]
       lea      rdx, [rbp-0x40]
       lea      rcx, [rbp-0xC8]
       mov      r8, gword ptr [rbp-0x1A0]
       mov      r9, r15
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinH_u(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       jmp      SHORT G_M000_IG85
 
G_M000_IG84:                ;; offset=0x0AB4
       mov      rdi, qword ptr [r14+0x28]
       mov      rsi, qword ptr [r14+0x18]
       lea      rdx, [rbp-0x40]
       lea      rcx, [rbp-0xC8]
       mov      r8, gword ptr [rbp-0x1A0]
       mov      r9, r15
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinR_u(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
 
G_M000_IG85:                ;; offset=0x0AD7
       mov      r15, rax
       mov      gword ptr [rbp-0x198], r14
       mov      r14, r15
       jmp      G_M000_IG48
 
G_M000_IG86:                ;; offset=0x0AE9
       mov      rdx, rdi
       cmp      r15d, dword ptr [rdx+0x10]
       jb       SHORT G_M000_IG88
 
G_M000_IG87:                ;; offset=0x0AF2
       call     [System.ThrowHelper:ThrowArgumentOutOfRange_IndexMustBeLessException()]
       int3     
 
G_M000_IG88:                ;; offset=0x0AF9
       mov      rax, r13
       mov      r13, r14
       mov      r14, rax
       mov      rdi, gword ptr [rdx+0x08]
       cmp      r15d, dword ptr [rdi+0x08]
       jae      G_M000_IG95
       mov      edx, r15d
       mov      rdi, qword ptr [rdi+8*rdx+0x10]
       mov      qword ptr [rbp-0x188], rdi
       lea      rdi, [rbp-0x188]
       call     [System.Runtime.InteropServices.GCHandle:Free():this]
       inc      r15d
       mov      rax, r13
       mov      r13, r14
       mov      r14, rax
       jmp      G_M000_IG42
 
G_M000_IG89:                ;; offset=0x0B3D
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      r15, rax
       jmp      G_M000_IG68
 
G_M000_IG90:                ;; offset=0x0B4F
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG69
 
G_M000_IG91:                ;; offset=0x0B5E
       mov      rax, -1
       test     r14, r14
       cmovge   r14, rax
       jmp      G_M000_IG49
 
G_M000_IG92:                ;; offset=0x0B74
       mov      eax, r14d
       jmp      G_M000_IG70
 
G_M000_IG93:                ;; offset=0x0B7C
       mov      rdi, qword ptr [r14+0x18]
       lea      rsi, [rbp-0x48]
       lea      rdx, [rbp-0x50]
       call     Armonik.Ffi.Harness.Abi:ak_enc_take(long,ulong,ulong):int
       test     eax, eax
       je       SHORT G_M000_IG94
       jmp      G_M000_IG70
 
G_M000_IG94:                ;; offset=0x0B96
       mov      rax, qword ptr [rbp-0x48]
       mov      r12, bword ptr [rbp-0x1A8]
       mov      qword ptr [r12], rax
       mov      eax, dword ptr [rbp-0x50]
       mov      rbx, bword ptr [rbp-0x1B0]
       mov      dword ptr [rbx], eax
       jmp      G_M000_IG78
 
G_M000_IG95:                ;; offset=0x0BB6
       call     CORINFO_HELP_RNGCHKFAIL
       int3     
 
; Total bytes of code 3004

