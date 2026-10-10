; Assembly listing for method Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:EncodeInto(Armonik.Ffi.Facade.UploadResultDataMessage,bool):int:this (Tier1)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1 code
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; fully interruptible
; with Dynamic PGO: edge weights are valid, and fgCalledCount is 6489
; 3 inlinees with PGO data; 1 single block inlinees; 0 inlinees without PGO data

G_M000_IG01:                ;; offset=0x0000
       push     rbp
       push     r15
       push     r14
       push     r13
       push     r12
       push     rbx
       sub      rsp, 344
       vzeroupper 
       lea      rbp, [rsp+0x180]
       xor      ecx, ecx
       mov      qword ptr [rbp-0x118], rcx
       vxorps   xmm8, xmm8, xmm8
       vmovdqa  xmmword ptr [rbp-0x110], xmm8
       vmovdqa  xmmword ptr [rbp-0x100], xmm8
       mov      rcx, -192
       vmovdqa  xmmword ptr [rbp+rcx-0x30], xmm8
       vmovdqa  xmmword ptr [rbp+rcx-0x20], xmm8
       vmovdqa  xmmword ptr [rbp+rcx-0x10], xmm8
       add      rcx, 48
       jne      SHORT  -5 instr
       mov      qword ptr [rbp-0x30], rcx
       mov      qword ptr [rbp-0x180], rsp
       mov      gword ptr [rbp-0x120], rdi
       mov      gword ptr [rbp-0x128], rsi
       mov      r15d, edx
 
G_M000_IG02:                ;; offset=0x0078
       lea      rdi, [rbp-0x170]
       mov      rsi, r10
       call     CORINFO_HELP_INIT_PINVOKE_FRAME
       mov      r14, rax
       mov      rdi, rsp
       mov      qword ptr [rbp-0x150], rdi
       mov      rdi, rbp
       mov      qword ptr [rbp-0x140], rdi
       mov      rax, gword ptr [rbp-0x120]
       mov      byte  ptr [rax+0x4C], 1
 
G_M000_IG03:                ;; offset=0x00A9
       mov      rdi, qword ptr [rax+0x18]
       mov      rcx, ADDR
       mov      qword ptr [rbp-0x160], rcx
       lea      rcx, G_M000_IG06
       mov      qword ptr [rbp-0x148], rcx
       lea      rcx, bword ptr [rbp-0x170]
       mov      qword ptr [r14+0x10], rcx
       mov      byte  ptr [r14+0x0C], 0
 
G_M000_IG04:                ;; offset=0x00DC
       mov      rax, ADDR
 
G_M000_IG05:                ;; offset=0x00E6
       call     rax ; Armonik.Ffi.Harness.Abi:ak_enc_reset(long)
 
G_M000_IG06:                ;; offset=0x00E8
       mov      byte  ptr [r14+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG07
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG07:                ;; offset=0x00FC
       mov      rdi, bword ptr [rbp-0x168]
       mov      qword ptr [r14+0x10], rdi
       mov      rax, gword ptr [rbp-0x120]
       mov      rbx, gword ptr [rax+0x08]
       cmp      byte  ptr [rbx], bl
       mov      rdi, rbx
       xor      esi, esi
       call     [Armonik.Ffi.Harness.Stage:Use(int):this]
       mov      rdi, rbx
       call     [Armonik.Ffi.Harness.Stage:ReleasePins():this]
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax+0x10], 3
       jl       G_M000_IG24
       mov      rdi, qword ptr [rax+0x18]
       mov      rdi, qword ptr [rdi+0x18]
       test     rdi, rdi
       je       G_M000_IG24
       mov      rax, bword ptr [rdi]
       add      rax, 16
 
G_M000_IG08:                ;; offset=0x0160
       mov      rdi, gword ptr [rax+0x08]
       test     rdi, rdi
       jne      G_M000_IG23
 
G_M000_IG09:                ;; offset=0x016D
       cmp      dword ptr [(reloc ADDR)], 7
       je       G_M000_IG22
 
G_M000_IG10:                ;; offset=0x017A
       cmp      dword ptr [(reloc ADDR)], 6
       jne      G_M000_IG21
 
G_M000_IG11:                ;; offset=0x0187
       mov      ebx, 1
 
G_M000_IG12:                ;; offset=0x018C
       xor      r13d, r13d
       xor      r12d, r12d
       test     ebx, ebx
       je       SHORT G_M000_IG15
 
G_M000_IG13:                ;; offset=0x0196
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax], 2
       jl       G_M000_IG20
       mov      rdx, qword ptr [rax+0x08]
       mov      r12, qword ptr [rdx+0x10]
       test     r12, r12
       je       G_M000_IG20
 
G_M000_IG14:                ;; offset=0x01C6
       mov      r13, qword ptr [r12+0x128]
       mov      r12, qword ptr [r12+0x130]
 
G_M000_IG15:                ;; offset=0x01D6
       mov      rax, gword ptr [rbp-0x120]
       mov      rdx, qword ptr [rax+0x28]
       mov      edi, dword ptr [rax+0x40]
       mov      dword ptr [rdx], edi
       mov      rdx, qword ptr [rax+0x28]
       xor      edi, edi
       test     r15b, r15b
       setne    dil
       mov      dword ptr [rdx+0x04], edi
       xor      edx, edx
       mov      qword ptr [rbp-0x30], rdx
       mov      rcx, gword ptr [rbp-0x128]
       mov      rdx, gword ptr [rcx+0x08]
       test     rdx, rdx
       je       G_M000_IG41
 
G_M000_IG16:                ;; offset=0x0210
       mov      r8, gword ptr [rdx+0x18]
 
G_M000_IG17:                ;; offset=0x0214
       test     r8, r8
       je       G_M000_IG40
 
G_M000_IG18:                ;; offset=0x021D
       mov      gword ptr [rbp-0x130], r8
       test     r15b, r15b
       jne      G_M000_IG39
       vxorps   ymm0, ymm0, ymm0
       vmovdqu32 zmmword ptr [rbp-0x88], zmm0
       vmovdqu  ymmword ptr [rbp-0x50], ymm0
       mov      rdx, gword ptr [rax+0x08]
       lea      rdi, [rbp-0x88]
       mov      gword ptr [rbp-0x128], rcx
       mov      rsi, rcx
       call     [Armonik.Ffi.Harness.G:E_UploadResultDataMessage(byref,Armonik.Ffi.Facade.UploadResultDataMessage,Armonik.Ffi.Harness.Stage)]
       mov      rax, ADDR
       inc      qword ptr [rax]
       test     ebx, ebx
       je       G_M000_IG44
 
G_M000_IG19:                ;; offset=0x0270
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax], 2
       jl       G_M000_IG38
       mov      rdi, qword ptr [rax+0x08]
       mov      rax, qword ptr [rdi+0x10]
       test     rax, rax
       je       G_M000_IG38
       jmp      G_M000_IG42
 
G_M000_IG20:                ;; offset=0x02A5
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      r12, rax
       jmp      G_M000_IG14
 
G_M000_IG21:                ;; offset=0x02B7
       mov      ebx, 2
       xor      edi, edi
       cmp      dword ptr [(reloc ADDR)], 7
       cmovne   ebx, edi
       jmp      G_M000_IG12
 
G_M000_IG22:                ;; offset=0x02CD
       xor      edi, edi
       call     [Armonik.Ffi.Harness.Stage:ReleaseChunk(int)]
       jmp      G_M000_IG10
 
G_M000_IG23:                ;; offset=0x02DA
       call     [System.Collections.Generic.List`1[System.__Canon]:Clear():this]
       jmp      G_M000_IG09
 
G_M000_IG24:                ;; offset=0x02E5
       mov      edi, 3
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG08
 
G_M000_IG25:                ;; offset=0x02F4
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       lea      rdi, bword ptr [rax+0x38]
       mov      rsi, gword ptr [rbp-0x128]
       call     CORINFO_HELP_ASSIGN_REF
       mov      dword ptr [r15+0x150], ebx
 
G_M000_IG26:                ;; offset=0x0315
       test     ebx, ebx
       jne      G_M000_IG35
       mov      r15, gword ptr [rbp-0x130]
       mov      gword ptr [rbp-0xA0], r15
       test     r15, r15
       je       SHORT G_M000_IG27
       mov      rdi, gword ptr [rbp-0xA0]
       cmp      dword ptr [rdi+0x08], 0
       jne      SHORT G_M000_IG28
 
G_M000_IG27:                ;; offset=0x033D
       xor      r8d, r8d
       jmp      SHORT G_M000_IG29
 
G_M000_IG28:                ;; offset=0x0342
       mov      r8, gword ptr [rbp-0xA0]
       cmp      dword ptr [r8+0x08], 0
       jbe      G_M000_IG34
       mov      r8, gword ptr [rbp-0xA0]
       add      r8, 16
 
G_M000_IG29:                ;; offset=0x035F
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, qword ptr [rax+0x28]
       mov      rsi, qword ptr [rax+0x18]
       mov      r9d, dword ptr [r15+0x08]
       lea      rcx, [rbp-0x118]
       lea      rdx, [rbp-0x30]
       mov      r10, ADDR
       mov      qword ptr [rbp-0x160], r10
       lea      r10, G_M000_IG31
       mov      qword ptr [rbp-0x148], r10
       lea      r10, bword ptr [rbp-0x170]
       mov      qword ptr [r14+0x10], r10
       mov      byte  ptr [r14+0x0C], 0
 
G_M000_IG30:                ;; offset=0x03AC
       call     [Armonik.Ffi.Harness.Abi:ak_uencode_UploadResultDataMessage(ulong,long,ulong,ulong,ulong,ulong):long]
 
G_M000_IG31:                ;; offset=0x03B2
       mov      byte  ptr [r14+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG32
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG32:                ;; offset=0x03C6
       mov      rdi, bword ptr [rbp-0x168]
       mov      qword ptr [r14+0x10], rdi
       mov      r15, rax
 
G_M000_IG33:                ;; offset=0x03D4
       xor      rdi, rdi
       mov      gword ptr [rbp-0xA0], rdi
       jmp      G_M000_IG46
 
G_M000_IG34:                ;; offset=0x03E2
       call     CORINFO_HELP_RNGCHKFAIL
 
G_M000_IG35:                ;; offset=0x03E7
       cmp      ebx, 1
       je       SHORT G_M000_IG36
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, qword ptr [rax+0x28]
       mov      rsi, qword ptr [rax+0x18]
       lea      rdx, [rbp-0x30]
       lea      rcx, [rbp-0x118]
       mov      r8, gword ptr [rbp-0x128]
       mov      r9, gword ptr [rbp-0x130]
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinH_u(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       mov      r15, rax
       jmp      SHORT G_M000_IG37
 
G_M000_IG36:                ;; offset=0x041F
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, qword ptr [rax+0x28]
       mov      rsi, qword ptr [rax+0x18]
       lea      rdx, [rbp-0x30]
       lea      rcx, [rbp-0x118]
       mov      r8, gword ptr [rbp-0x128]
       mov      r9, gword ptr [rbp-0x130]
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinR_u(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       mov      r15, rax
 
G_M000_IG37:                ;; offset=0x0450
       jmp      G_M000_IG46
 
G_M000_IG38:                ;; offset=0x0455
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG42
 
G_M000_IG39:                ;; offset=0x0464
       vxorps   ymm0, ymm0, ymm0
       vmovdqu32 zmmword ptr [rbp-0x118], zmm0
       vmovdqu32 zmmword ptr [rbp-0xE0], zmm0
       mov      rdx, gword ptr [rax+0x08]
       lea      rdi, [rbp-0x118]
       mov      gword ptr [rbp-0x128], rcx
       mov      rsi, rcx
       call     [Armonik.Ffi.Harness.G:U_UploadResultDataMessage(byref,Armonik.Ffi.Facade.UploadResultDataMessage,Armonik.Ffi.Harness.Stage)]
       mov      rax, ADDR
       inc      qword ptr [rax]
       test     ebx, ebx
       je       G_M000_IG26
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      r15, rax
       cmp      qword ptr [r15+0x128], r13
       jne      G_M000_IG25
       xor      ebx, ebx
       jmp      G_M000_IG26
 
G_M000_IG40:                ;; offset=0x04CD
       mov      gword ptr [rbp-0x128], rcx
       mov      rdi, ADDR
       mov      esi, 11
       call     CORINFO_HELP_CLASSINIT_SHARED_DYNAMICCLASS
       mov      rdx, ADDR
       mov      r8, gword ptr [rdx]
       mov      rdx, r8
       mov      r8, rdx
       mov      rax, gword ptr [rbp-0x120]
       mov      rcx, gword ptr [rbp-0x128]
       jmp      G_M000_IG18
 
G_M000_IG41:                ;; offset=0x050E
       xor      r8, r8
       jmp      G_M000_IG17
 
G_M000_IG42:                ;; offset=0x0516
       mov      r15, rax
       cmp      qword ptr [r15+0x128], r13
       je       G_M000_IG54
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
 
G_M000_IG43:                ;; offset=0x055E
       lea      rdi, bword ptr [rax+0x38]
       mov      rsi, gword ptr [rbp-0x128]
       call     CORINFO_HELP_ASSIGN_REF
       mov      dword ptr [r15+0x150], ebx
 
G_M000_IG44:                ;; offset=0x0575
       test     ebx, ebx
       je       G_M000_IG51
       cmp      ebx, 1
       jne      G_M000_IG67
 
G_M000_IG45:                ;; offset=0x0586
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, qword ptr [rax+0x28]
       mov      rsi, qword ptr [rax+0x18]
       lea      rdx, [rbp-0x30]
       lea      rcx, [rbp-0x88]
       mov      r8, gword ptr [rbp-0x128]
       mov      r9, gword ptr [rbp-0x130]
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinR_e(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       mov      r15, rax
 
G_M000_IG46:                ;; offset=0x05B7
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, gword ptr [rax+0x08]
       cmp      dword ptr [rdi], edi
       call     [Armonik.Ffi.Harness.Stage:ReleasePins():this]
       test     ebx, ebx
       je       G_M000_IG69
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax], 2
       jl       SHORT G_M000_IG48
       mov      rdi, qword ptr [rax+0x08]
       mov      rax, qword ptr [rdi+0x10]
       test     rax, rax
       je       SHORT G_M000_IG48
 
G_M000_IG47:                ;; offset=0x05FA
       mov      rbx, rax
       xor      edi, edi
       mov      dword ptr [rbx+0x150], edi
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax+0x10], 2
       jl       G_M000_IG61
       mov      rax, qword ptr [rax+0x18]
       mov      rax, qword ptr [rax+0x10]
       test     rax, rax
       je       G_M000_IG61
       mov      rcx, bword ptr [rax]
       add      rcx, 16
       jmp      G_M000_IG68
 
G_M000_IG48:                ;; offset=0x0642
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      SHORT G_M000_IG47
 
G_M000_IG49:                ;; offset=0x064E
       mov      edi, r15d
       mov      rax, gword ptr [rbp-0x120]
       jmp      G_M000_IG71
 
G_M000_IG50:                ;; offset=0x065D
       mov      rdi, -1
       test     r15, r15
       cmovge   r15, rdi
       jmp      G_M000_IG69
 
G_M000_IG51:                ;; offset=0x0673
       mov      r15, gword ptr [rbp-0x130]
       mov      gword ptr [rbp-0xA0], r15
       test     r15, r15
       je       SHORT G_M000_IG52
       mov      rdi, gword ptr [rbp-0xA0]
       cmp      dword ptr [rdi+0x08], 0
       jne      SHORT G_M000_IG55
 
G_M000_IG52:                ;; offset=0x0693
       xor      r8d, r8d
       jmp      SHORT G_M000_IG56
 
G_M000_IG53:                ;; offset=0x0698
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG43
 
G_M000_IG54:                ;; offset=0x06A7
       xor      ebx, ebx
       jmp      G_M000_IG44
 
G_M000_IG55:                ;; offset=0x06AE
       mov      r8, gword ptr [rbp-0xA0]
       cmp      dword ptr [r8+0x08], 0
       jbe      G_M000_IG34
       mov      r8, gword ptr [rbp-0xA0]
       add      r8, 16
 
G_M000_IG56:                ;; offset=0x06CB
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, qword ptr [rax+0x28]
       mov      rsi, qword ptr [rax+0x18]
       mov      r9d, dword ptr [r15+0x08]
       lea      rcx, [rbp-0x88]
       lea      rdx, [rbp-0x30]
       mov      r10, ADDR
       mov      qword ptr [rbp-0x160], r10
       lea      r10, G_M000_IG59
       mov      qword ptr [rbp-0x148], r10
       lea      r10, bword ptr [rbp-0x170]
       mov      qword ptr [r14+0x10], r10
       mov      byte  ptr [r14+0x0C], 0
 
G_M000_IG57:                ;; offset=0x0718
       mov      rax, ADDR
 
G_M000_IG58:                ;; offset=0x0722
       call     rax ; Armonik.Ffi.Harness.Abi:ak_encode_UploadResultDataMessage(ulong,long,ulong,ulong,ulong,ulong):long
 
G_M000_IG59:                ;; offset=0x0724
       mov      byte  ptr [r14+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG60
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG60:                ;; offset=0x0738
       mov      rdi, bword ptr [rbp-0x168]
       mov      qword ptr [r14+0x10], rdi
       mov      r15, rax
       jmp      G_M000_IG33
 
G_M000_IG61:                ;; offset=0x074B
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      rcx, rax
       jmp      G_M000_IG68
 
G_M000_IG62:                ;; offset=0x075D
       mov      rdi, qword ptr [rax+0x18]
       lea      rsi, [rbp-0x90]
       lea      rdx, [rbp-0x98]
       mov      rcx, ADDR
       mov      qword ptr [rbp-0x160], rcx
       lea      rcx, G_M000_IG65
       mov      qword ptr [rbp-0x148], rcx
       lea      rcx, bword ptr [rbp-0x170]
       mov      qword ptr [r14+0x10], rcx
       mov      byte  ptr [r14+0x0C], 0
 
G_M000_IG63:                ;; offset=0x079E
       mov      rax, ADDR
 
G_M000_IG64:                ;; offset=0x07A8
       call     rax ; Armonik.Ffi.Harness.Abi:ak_enc_take(long,ulong,ulong):int
 
G_M000_IG65:                ;; offset=0x07AA
       mov      byte  ptr [r14+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG66
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG66:                ;; offset=0x07BE
       mov      rcx, bword ptr [rbp-0x168]
       mov      qword ptr [r14+0x10], rcx
       mov      edi, eax
       test     edi, edi
       mov      rax, gword ptr [rbp-0x120]
       je       SHORT G_M000_IG70
       jmp      SHORT G_M000_IG71
 
G_M000_IG67:                ;; offset=0x07D8
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, qword ptr [rax+0x28]
       mov      rsi, qword ptr [rax+0x18]
       lea      rdx, [rbp-0x30]
       lea      rcx, [rbp-0x88]
       mov      r8, gword ptr [rbp-0x128]
       mov      r9, gword ptr [rbp-0x130]
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinH_e(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       mov      r15, rax
       jmp      G_M000_IG46
 
G_M000_IG68:                ;; offset=0x080E
       xor      rax, rax
       mov      gword ptr [rcx+0x38], rax
       mov      rax, qword ptr [rbx+0x128]
       sub      rax, r13
       mov      rdi, qword ptr [rbx+0x130]
       sub      rdi, r12
       cmp      rax, rdi
       jne      G_M000_IG50
 
G_M000_IG69:                ;; offset=0x0831
       test     r15, r15
       jl       G_M000_IG49
       mov      rax, gword ptr [rbp-0x120]
       cmp      byte  ptr [rax+0x4C], 0
       je       G_M000_IG62
 
G_M000_IG70:                ;; offset=0x084B
       xor      edi, edi
 
G_M000_IG71:                ;; offset=0x084D
       xor      rcx, rcx
       mov      gword ptr [rbp-0xA0], rcx
 
G_M000_IG72:                ;; offset=0x0856
       mov      byte  ptr [rax+0x4C], 0
       mov      eax, edi
 
G_M000_IG73:                ;; offset=0x085C
       add      rsp, 344
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG74:                ;; offset=0x086E
       push     rbp
       push     r15
       push     r14
       push     r13
       push     r12
       push     rbx
       push     rax
       vzeroupper 
       mov      rbp, qword ptr [rdi]
       mov      qword ptr [rsp], rbp
       lea      rbp, [rbp+0x180]
 
G_M000_IG75:                ;; offset=0x088A
       mov      rax, gword ptr [rbp-0x120]
       mov      byte  ptr [rax+0x4C], 0
 
G_M000_IG76:                ;; offset=0x0895
       add      rsp, 8
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
; Total bytes of code 2212

