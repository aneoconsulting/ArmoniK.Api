; Assembly listing for method Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:EncodeInto(Armonik.Ffi.Facade.UploadResultDataMessage,bool):int:this (Tier1)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1 code
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; fully interruptible
; with Dynamic PGO: edge weights are invalid, and fgCalledCount is 29216
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
       jl       G_M000_IG42
       mov      rdi, qword ptr [rax+0x18]
       mov      rdi, qword ptr [rdi+0x18]
       test     rdi, rdi
       je       G_M000_IG42
       mov      rax, bword ptr [rdi]
       add      rax, 16
 
G_M000_IG08:                ;; offset=0x0160
       mov      rdi, gword ptr [rax+0x08]
       test     rdi, rdi
       jne      G_M000_IG41
 
G_M000_IG09:                ;; offset=0x016D
       cmp      dword ptr [(reloc ADDR)], 7
       je       G_M000_IG40
 
G_M000_IG10:                ;; offset=0x017A
       mov      edi, dword ptr [(reloc ADDR)]
       mov      ebx, 1
       mov      eax, 2
       xor      ecx, ecx
       cmp      edi, 7
       cmovne   eax, ecx
       cmp      edi, 6
       cmovne   ebx, eax
 
G_M000_IG11:                ;; offset=0x0198
       xor      r13d, r13d
 
G_M000_IG12:                ;; offset=0x019B
       xor      r12d, r12d
       test     ebx, ebx
       je       SHORT G_M000_IG15
 
G_M000_IG13:                ;; offset=0x01A2
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax], 2
       jl       G_M000_IG39
       mov      rdx, qword ptr [rax+0x08]
       mov      r12, qword ptr [rdx+0x10]
       test     r12, r12
       je       G_M000_IG39
 
G_M000_IG14:                ;; offset=0x01D2
       mov      r13, qword ptr [r12+0x128]
       mov      r12, qword ptr [r12+0x130]
 
G_M000_IG15:                ;; offset=0x01E2
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
       je       G_M000_IG75
 
G_M000_IG16:                ;; offset=0x021C
       mov      r8, gword ptr [rdx+0x18]
 
G_M000_IG17:                ;; offset=0x0220
       test     r8, r8
       je       G_M000_IG62
 
G_M000_IG18:                ;; offset=0x0229
       mov      gword ptr [rbp-0x130], r8
       test     r15b, r15b
       jne      G_M000_IG68
 
G_M000_IG19:                ;; offset=0x0239
       vxorps   ymm0, ymm0, ymm0
       vmovdqu32 zmmword ptr [rbp-0x88], zmm0
       vmovdqu  ymmword ptr [rbp-0x50], ymm0
       mov      rdx, gword ptr [rax+0x08]
       lea      rdi, [rbp-0x88]
       mov      gword ptr [rbp-0x128], rcx
       mov      rsi, rcx
       call     [Armonik.Ffi.Harness.G:E_UploadResultDataMessage(byref,Armonik.Ffi.Facade.UploadResultDataMessage,Armonik.Ffi.Harness.Stage)]
       mov      r15, ADDR
       inc      qword ptr [r15]
       test     ebx, ebx
       je       G_M000_IG23
 
G_M000_IG20:                ;; offset=0x027C
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax], 2
       jl       G_M000_IG54
       mov      rdi, qword ptr [rax+0x08]
       mov      rax, qword ptr [rdi+0x10]
       test     rax, rax
       je       G_M000_IG54
 
G_M000_IG21:                ;; offset=0x02AC
       mov      r15, rax
       cmp      qword ptr [r15+0x128], r13
       je       G_M000_IG53
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax+0x10], 2
       jl       G_M000_IG52
       mov      rdi, qword ptr [rax+0x18]
       mov      rdi, qword ptr [rdi+0x10]
       test     rdi, rdi
       je       G_M000_IG52
       mov      rax, bword ptr [rdi]
       add      rax, 16
 
G_M000_IG22:                ;; offset=0x02F4
       lea      rdi, bword ptr [rax+0x38]
       mov      rsi, gword ptr [rbp-0x128]
       call     CORINFO_HELP_ASSIGN_REF
       mov      dword ptr [r15+0x150], ebx
 
G_M000_IG23:                ;; offset=0x030B
       test     ebx, ebx
       jne      G_M000_IG63
 
G_M000_IG24:                ;; offset=0x0313
       mov      r15, gword ptr [rbp-0x130]
       mov      gword ptr [rbp-0xA0], r15
       test     r15, r15
       je       SHORT G_M000_IG26
 
G_M000_IG25:                ;; offset=0x0326
       mov      rdi, gword ptr [rbp-0xA0]
       cmp      dword ptr [rdi+0x08], 0
       jne      G_M000_IG55
 
G_M000_IG26:                ;; offset=0x0337
       xor      r8d, r8d
 
G_M000_IG27:                ;; offset=0x033A
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, qword ptr [rax+0x28]
       mov      rsi, qword ptr [rax+0x18]
       mov      r9d, dword ptr [r15+0x08]
       lea      rcx, [rbp-0x88]
       lea      rdx, [rbp-0x30]
       mov      r10, ADDR
       mov      qword ptr [rbp-0x160], r10
       lea      r10, G_M000_IG30
       mov      qword ptr [rbp-0x148], r10
       lea      r10, bword ptr [rbp-0x170]
       mov      qword ptr [r14+0x10], r10
       mov      byte  ptr [r14+0x0C], 0
 
G_M000_IG28:                ;; offset=0x0387
       mov      rax, ADDR
 
G_M000_IG29:                ;; offset=0x0391
       call     rax ; Armonik.Ffi.Harness.Abi:ak_encode_UploadResultDataMessage(ulong,long,ulong,ulong,ulong,ulong):long
 
G_M000_IG30:                ;; offset=0x0393
       mov      byte  ptr [r14+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG31
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG31:                ;; offset=0x03A7
       mov      rdi, bword ptr [rbp-0x168]
       mov      qword ptr [r14+0x10], rdi
       mov      r15, rax
 
G_M000_IG32:                ;; offset=0x03B5
       xor      rdx, rdx
       mov      gword ptr [rbp-0xA0], rdx
 
G_M000_IG33:                ;; offset=0x03BE
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, gword ptr [rax+0x08]
       cmp      dword ptr [rdi], edi
       call     [Armonik.Ffi.Harness.Stage:ReleasePins():this]
       test     ebx, ebx
       je       G_M000_IG37
 
G_M000_IG34:                ;; offset=0x03D9
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax], 2
       jl       G_M000_IG66
       mov      rdi, qword ptr [rax+0x08]
       mov      rax, qword ptr [rdi+0x10]
       test     rax, rax
       je       G_M000_IG66
 
G_M000_IG35:                ;; offset=0x0409
       mov      rbx, rax
       xor      edi, edi
       mov      dword ptr [rbx+0x150], edi
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax+0x10], 2
       jl       G_M000_IG49
       mov      rax, qword ptr [rax+0x18]
       mov      rax, qword ptr [rax+0x10]
       test     rax, rax
       je       G_M000_IG49
       mov      rcx, bword ptr [rax]
       add      rcx, 16
 
G_M000_IG36:                ;; offset=0x044C
       xor      rax, rax
       mov      gword ptr [rcx+0x38], rax
       mov      rax, qword ptr [rbx+0x128]
       sub      rax, r13
       mov      rdi, qword ptr [rbx+0x130]
       sub      rdi, r12
       cmp      rax, rdi
       jne      G_M000_IG50
 
G_M000_IG37:                ;; offset=0x046F
       test     r15, r15
       jl       G_M000_IG51
       mov      rax, gword ptr [rbp-0x120]
       cmp      byte  ptr [rax+0x4C], 0
       je       G_M000_IG56
 
G_M000_IG38:                ;; offset=0x0489
       xor      edi, edi
       jmp      G_M000_IG76
 
G_M000_IG39:                ;; offset=0x0490
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      r12, rax
       jmp      G_M000_IG14
 
G_M000_IG40:                ;; offset=0x04A2
       xor      edi, edi
       call     [Armonik.Ffi.Harness.Stage:ReleaseChunk(int)]
       jmp      G_M000_IG10
 
G_M000_IG41:                ;; offset=0x04AF
       call     [System.Collections.Generic.List`1[System.__Canon]:Clear():this]
       jmp      G_M000_IG09
 
G_M000_IG42:                ;; offset=0x04BA
       mov      edi, 3
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG08
 
G_M000_IG43:                ;; offset=0x04C9
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       lea      rdi, bword ptr [rax+0x38]
       mov      rsi, gword ptr [rbp-0x128]
       call     CORINFO_HELP_ASSIGN_REF
       mov      dword ptr [r15+0x150], ebx
       jmp      G_M000_IG69
 
G_M000_IG44:                ;; offset=0x04EF
       mov      rdi, gword ptr [rbp-0xA0]
       cmp      dword ptr [rdi+0x08], 0
       je       G_M000_IG70
       mov      rdi, gword ptr [rbp-0xA0]
       cmp      dword ptr [rdi+0x08], 0
       jbe      SHORT G_M000_IG45
       mov      r8, gword ptr [rbp-0xA0]
       add      r8, 16
       jmp      G_M000_IG71
 
G_M000_IG45:                ;; offset=0x051D
       call     CORINFO_HELP_RNGCHKFAIL
 
G_M000_IG46:                ;; offset=0x0522
       mov      r15, gword ptr [rbp-0x130]
       cmp      ebx, 1
       je       SHORT G_M000_IG47
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, qword ptr [rax+0x28]
       mov      rsi, qword ptr [rax+0x18]
       lea      rdx, [rbp-0x30]
       lea      rcx, [rbp-0x118]
       mov      r8, gword ptr [rbp-0x128]
       mov      r9, r15
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinH_u(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       mov      r15, rax
       jmp      SHORT G_M000_IG48
 
G_M000_IG47:                ;; offset=0x055D
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, qword ptr [rax+0x28]
       mov      rsi, qword ptr [rax+0x18]
       lea      rdx, [rbp-0x30]
       lea      rcx, [rbp-0x118]
       mov      r8, gword ptr [rbp-0x128]
       mov      r9, r15
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinR_u(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       mov      r15, rax
 
G_M000_IG48:                ;; offset=0x058A
       jmp      G_M000_IG33
 
G_M000_IG49:                ;; offset=0x058F
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      rcx, rax
       jmp      G_M000_IG36
 
G_M000_IG50:                ;; offset=0x05A1
       mov      rdi, -1
       test     r15, r15
       cmovge   r15, rdi
       jmp      G_M000_IG37
 
G_M000_IG51:                ;; offset=0x05B7
       mov      edi, r15d
       mov      rax, gword ptr [rbp-0x120]
       jmp      G_M000_IG76
 
G_M000_IG52:                ;; offset=0x05C6
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG22
 
G_M000_IG53:                ;; offset=0x05D5
       xor      ebx, ebx
       jmp      G_M000_IG23
 
G_M000_IG54:                ;; offset=0x05DC
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG21
 
G_M000_IG55:                ;; offset=0x05EB
       mov      rdi, gword ptr [rbp-0xA0]
       cmp      dword ptr [rdi+0x08], 0
       jbe      G_M000_IG45
       mov      r8, gword ptr [rbp-0xA0]
       add      r8, 16
       jmp      G_M000_IG27
 
G_M000_IG56:                ;; offset=0x060C
       mov      rdi, qword ptr [rax+0x18]
       lea      rsi, [rbp-0x90]
       lea      rdx, [rbp-0x98]
       mov      rcx, ADDR
       mov      qword ptr [rbp-0x160], rcx
       lea      rcx, G_M000_IG59
       mov      qword ptr [rbp-0x148], rcx
       lea      rcx, bword ptr [rbp-0x170]
       mov      qword ptr [r14+0x10], rcx
       mov      byte  ptr [r14+0x0C], 0
 
G_M000_IG57:                ;; offset=0x064D
       mov      rax, ADDR
 
G_M000_IG58:                ;; offset=0x0657
       call     rax ; Armonik.Ffi.Harness.Abi:ak_enc_take(long,ulong,ulong):int
 
G_M000_IG59:                ;; offset=0x0659
       mov      byte  ptr [r14+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG60
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG60:                ;; offset=0x066D
       mov      rcx, bword ptr [rbp-0x168]
       mov      qword ptr [r14+0x10], rcx
       mov      edi, eax
       test     edi, edi
       mov      rax, gword ptr [rbp-0x120]
       je       G_M000_IG38
       jmp      G_M000_IG76
 
G_M000_IG61:                ;; offset=0x068E
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      r15, rax
       cmp      qword ptr [r15+0x128], r13
       jne      G_M000_IG43
       xor      ebx, ebx
       jmp      G_M000_IG69
 
G_M000_IG62:                ;; offset=0x06AF
       mov      gword ptr [rbp-0x128], rcx
       mov      rdi, ADDR
       mov      esi, 11
       call     CORINFO_HELP_CLASSINIT_SHARED_DYNAMICCLASS
       mov      rdi, ADDR
       mov      r8, gword ptr [rdi]
       mov      rdi, r8
       mov      r8, rdi
       mov      rax, gword ptr [rbp-0x120]
       mov      rcx, gword ptr [rbp-0x128]
       jmp      G_M000_IG18
 
G_M000_IG63:                ;; offset=0x06F0
       cmp      ebx, 1
       jne      SHORT G_M000_IG67
 
G_M000_IG64:                ;; offset=0x06F5
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, qword ptr [rax+0x28]
       mov      rsi, qword ptr [rax+0x18]
       lea      rdx, [rbp-0x30]
       lea      rcx, [rbp-0x88]
       mov      r8, gword ptr [rbp-0x128]
       mov      r9, gword ptr [rbp-0x130]
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinR_e(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       mov      r15, rax
 
G_M000_IG65:                ;; offset=0x0726
       jmp      G_M000_IG33
 
G_M000_IG66:                ;; offset=0x072B
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG35
 
G_M000_IG67:                ;; offset=0x073A
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, qword ptr [rax+0x28]
       mov      rsi, qword ptr [rax+0x18]
       lea      rdx, [rbp-0x30]
       lea      rcx, [rbp-0x88]
       mov      r8, gword ptr [rbp-0x128]
       mov      r9, gword ptr [rbp-0x130]
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinH_e(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       mov      r15, rax
       jmp      SHORT G_M000_IG65
 
G_M000_IG68:                ;; offset=0x076D
       vxorps   ymm2, ymm2, ymm2
       vmovdqu32 zmmword ptr [rbp-0x118], zmm2
       vmovdqu32 zmmword ptr [rbp-0xE0], zmm2
       mov      rdx, gword ptr [rax+0x08]
       lea      rdi, [rbp-0x118]
       mov      gword ptr [rbp-0x128], rcx
       mov      rsi, rcx
       call     [Armonik.Ffi.Harness.G:U_UploadResultDataMessage(byref,Armonik.Ffi.Facade.UploadResultDataMessage,Armonik.Ffi.Harness.Stage)]
       mov      r15, ADDR
       inc      qword ptr [r15]
       test     ebx, ebx
       jne      G_M000_IG61
 
G_M000_IG69:                ;; offset=0x07B5
       test     ebx, ebx
       jne      G_M000_IG46
       mov      r15, gword ptr [rbp-0x130]
       mov      gword ptr [rbp-0xA0], r15
       test     r15, r15
       jne      G_M000_IG44
 
G_M000_IG70:                ;; offset=0x07D4
       xor      r8d, r8d
 
G_M000_IG71:                ;; offset=0x07D7
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, qword ptr [rax+0x28]
       mov      rsi, qword ptr [rax+0x18]
       mov      r9d, dword ptr [r15+0x08]
       lea      rcx, [rbp-0x118]
       lea      rdx, [rbp-0x30]
       mov      r10, ADDR
       mov      qword ptr [rbp-0x160], r10
       lea      r10, G_M000_IG73
       mov      qword ptr [rbp-0x148], r10
       lea      r10, bword ptr [rbp-0x170]
       mov      qword ptr [r14+0x10], r10
       mov      byte  ptr [r14+0x0C], 0
 
G_M000_IG72:                ;; offset=0x0824
       call     [Armonik.Ffi.Harness.Abi:ak_uencode_UploadResultDataMessage(ulong,long,ulong,ulong,ulong,ulong):long]
 
G_M000_IG73:                ;; offset=0x082A
       mov      byte  ptr [r14+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG74
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG74:                ;; offset=0x083E
       mov      rdx, bword ptr [rbp-0x168]
       mov      qword ptr [r14+0x10], rdx
       mov      r15, rax
       jmp      G_M000_IG32
 
G_M000_IG75:                ;; offset=0x0851
       xor      r8, r8
       jmp      G_M000_IG17
 
G_M000_IG76:                ;; offset=0x0859
       xor      rcx, rcx
       mov      gword ptr [rbp-0xA0], rcx
 
G_M000_IG77:                ;; offset=0x0862
       mov      byte  ptr [rax+0x4C], 0
       mov      eax, edi
 
G_M000_IG78:                ;; offset=0x0868
       add      rsp, 344
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG79:                ;; offset=0x087A
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
 
G_M000_IG80:                ;; offset=0x0896
       mov      rax, gword ptr [rbp-0x120]
       mov      byte  ptr [rax+0x4C], 0
 
G_M000_IG81:                ;; offset=0x08A1
       add      rsp, 8
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
; Total bytes of code 2224

