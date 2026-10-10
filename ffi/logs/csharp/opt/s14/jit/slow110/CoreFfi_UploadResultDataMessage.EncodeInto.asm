; Assembly listing for method Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:EncodeInto(Armonik.Ffi.Facade.UploadResultDataMessage,bool):int:this (Tier1)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1 code
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; fully interruptible
; with Dynamic PGO: edge weights are invalid, and fgCalledCount is 21540
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
       jl       G_M000_IG35
       mov      rdx, qword ptr [rax+0x18]
       mov      rdx, qword ptr [rdx+0x18]
       test     rdx, rdx
       je       G_M000_IG35
       mov      rax, bword ptr [rdx]
       add      rax, 16
 
G_M000_IG08:                ;; offset=0x0160
       mov      rdi, gword ptr [rax+0x08]
       test     rdi, rdi
       jne      G_M000_IG34
 
G_M000_IG09:                ;; offset=0x016D
       cmp      dword ptr [(reloc ADDR)], 7
       je       G_M000_IG33
 
G_M000_IG10:                ;; offset=0x017A
       mov      edx, dword ptr [(reloc ADDR)]
       mov      ebx, 1
       mov      edi, 2
       xor      esi, esi
       cmp      edx, 7
       cmovne   edi, esi
       cmp      edx, 6
       cmovne   ebx, edi
 
G_M000_IG11:                ;; offset=0x0198
       xor      r13d, r13d
 
G_M000_IG12:                ;; offset=0x019B
       xor      r12d, r12d
       test     ebx, ebx
       jne      G_M000_IG32
 
G_M000_IG13:                ;; offset=0x01A6
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
       je       G_M000_IG62
 
G_M000_IG14:                ;; offset=0x01E0
       mov      r8, gword ptr [rdx+0x18]
 
G_M000_IG15:                ;; offset=0x01E4
       test     r8, r8
       je       G_M000_IG31
 
G_M000_IG16:                ;; offset=0x01ED
       mov      gword ptr [rbp-0x130], r8
       test     r15b, r15b
       jne      G_M000_IG55
 
G_M000_IG17:                ;; offset=0x01FD
       vxorps   ymm2, ymm2, ymm2
       vmovdqu32 zmmword ptr [rbp-0x88], zmm2
       vmovdqu  ymmword ptr [rbp-0x50], ymm2
       mov      rdx, gword ptr [rax+0x08]
       lea      rdi, [rbp-0x88]
       mov      gword ptr [rbp-0x128], rcx
       mov      rsi, rcx
       call     [Armonik.Ffi.Harness.G:E_UploadResultDataMessage(byref,Armonik.Ffi.Facade.UploadResultDataMessage,Armonik.Ffi.Harness.Stage)]
       mov      r15, ADDR
       inc      qword ptr [r15]
       test     ebx, ebx
       jne      G_M000_IG44
 
G_M000_IG18:                ;; offset=0x0240
       test     ebx, ebx
       jne      G_M000_IG47
       mov      r15, gword ptr [rbp-0x130]
       mov      gword ptr [rbp-0xA0], r15
       test     r15, r15
       je       SHORT G_M000_IG20
 
G_M000_IG19:                ;; offset=0x025B
       mov      rdi, gword ptr [rbp-0xA0]
       cmp      dword ptr [rdi+0x08], 0
       jne      G_M000_IG46
 
G_M000_IG20:                ;; offset=0x026C
       xor      r8d, r8d
 
G_M000_IG21:                ;; offset=0x026F
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, qword ptr [rax+0x28]
       mov      rsi, qword ptr [rax+0x18]
       mov      r9d, dword ptr [r15+0x08]
       lea      rcx, [rbp-0x88]
       lea      rdx, [rbp-0x30]
       mov      r10, ADDR
       mov      qword ptr [rbp-0x160], r10
       lea      r10, G_M000_IG24
       mov      qword ptr [rbp-0x148], r10
       lea      r10, bword ptr [rbp-0x170]
       mov      qword ptr [r14+0x10], r10
       mov      byte  ptr [r14+0x0C], 0
 
G_M000_IG22:                ;; offset=0x02BC
       mov      rax, ADDR
 
G_M000_IG23:                ;; offset=0x02C6
       call     rax ; Armonik.Ffi.Harness.Abi:ak_encode_UploadResultDataMessage(ulong,long,ulong,ulong,ulong,ulong):long
 
G_M000_IG24:                ;; offset=0x02C8
       mov      byte  ptr [r14+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG25
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG25:                ;; offset=0x02DC
       mov      rdi, bword ptr [rbp-0x168]
       mov      qword ptr [r14+0x10], rdi
       mov      r15, rax
 
G_M000_IG26:                ;; offset=0x02EA
       xor      rdx, rdx
       mov      gword ptr [rbp-0xA0], rdx
 
G_M000_IG27:                ;; offset=0x02F3
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, gword ptr [rax+0x08]
       cmp      dword ptr [rdi], edi
       call     [Armonik.Ffi.Harness.Stage:ReleasePins():this]
       test     ebx, ebx
       jne      G_M000_IG43
 
G_M000_IG28:                ;; offset=0x030E
       test     r15, r15
       jl       G_M000_IG42
       mov      rax, gword ptr [rbp-0x120]
       cmp      byte  ptr [rax+0x4C], 0
       je       G_M000_IG50
 
G_M000_IG29:                ;; offset=0x0328
       xor      edi, edi
       jmp      G_M000_IG63
 
G_M000_IG30:                ;; offset=0x032F
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       cmp      qword ptr [rax+0x128], r13
       jne      G_M000_IG36
       xor      ebx, ebx
       jmp      G_M000_IG56
 
G_M000_IG31:                ;; offset=0x034D
       mov      gword ptr [rbp-0x128], rcx
       mov      rdi, ADDR
       mov      esi, 11
       call     CORINFO_HELP_CLASSINIT_SHARED_DYNAMICCLASS
       mov      rdi, ADDR
       mov      r8, gword ptr [rdi]
       mov      rax, gword ptr [rbp-0x120]
       mov      rcx, gword ptr [rbp-0x128]
       jmp      G_M000_IG16
 
G_M000_IG32:                ;; offset=0x0388
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      r13, qword ptr [rax+0x128]
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      r12, qword ptr [rax+0x130]
       jmp      G_M000_IG13
 
G_M000_IG33:                ;; offset=0x03AF
       xor      edi, edi
       call     [Armonik.Ffi.Harness.Stage:ReleaseChunk(int)]
       jmp      G_M000_IG10
 
G_M000_IG34:                ;; offset=0x03BC
       call     [System.Collections.Generic.List`1[System.__Canon]:Clear():this]
       jmp      G_M000_IG09
 
G_M000_IG35:                ;; offset=0x03C7
       mov      edi, 3
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG08
 
G_M000_IG36:                ;; offset=0x03D6
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       lea      rdi, bword ptr [rax+0x38]
       mov      rsi, gword ptr [rbp-0x128]
       call     CORINFO_HELP_ASSIGN_REF
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      dword ptr [rax+0x150], ebx
       jmp      G_M000_IG56
 
G_M000_IG37:                ;; offset=0x0405
       mov      rdi, gword ptr [rbp-0xA0]
       cmp      dword ptr [rdi+0x08], 0
       je       G_M000_IG57
       mov      rdi, gword ptr [rbp-0xA0]
       cmp      dword ptr [rdi+0x08], 0
       jbe      SHORT G_M000_IG38
       mov      r8, gword ptr [rbp-0xA0]
       add      r8, 16
       jmp      G_M000_IG58
 
G_M000_IG38:                ;; offset=0x0433
       call     CORINFO_HELP_RNGCHKFAIL
 
G_M000_IG39:                ;; offset=0x0438
       mov      r15, gword ptr [rbp-0x130]
       cmp      ebx, 1
       je       SHORT G_M000_IG40
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, qword ptr [rax+0x28]
       mov      rsi, qword ptr [rax+0x18]
       lea      rdx, [rbp-0x30]
       lea      rcx, [rbp-0x118]
       mov      r8, gword ptr [rbp-0x128]
       mov      r9, r15
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinH_u(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       mov      r15, rax
       jmp      SHORT G_M000_IG41
 
G_M000_IG40:                ;; offset=0x0473
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, qword ptr [rax+0x28]
       mov      rsi, qword ptr [rax+0x18]
       lea      rdx, [rbp-0x30]
       lea      rcx, [rbp-0x118]
       mov      r8, gword ptr [rbp-0x128]
       mov      r9, r15
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinR_u(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       mov      r15, rax
 
G_M000_IG41:                ;; offset=0x04A0
       jmp      G_M000_IG27
 
G_M000_IG42:                ;; offset=0x04A5
       mov      edi, r15d
       mov      rax, gword ptr [rbp-0x120]
       jmp      G_M000_IG63
 
G_M000_IG43:                ;; offset=0x04B4
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       xor      edi, edi
       mov      dword ptr [rax+0x150], edi
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       xor      rdi, rdi
       mov      gword ptr [rax+0x38], rdi
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      rbx, qword ptr [rax+0x128]
       sub      rbx, r13
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      rdi, qword ptr [rax+0x130]
       sub      rdi, r12
       cmp      rbx, rdi
       je       G_M000_IG28
       mov      rdi, -1
       test     r15, r15
       cmovge   r15, rdi
       jmp      G_M000_IG28
 
G_M000_IG44:                ;; offset=0x051D
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       cmp      qword ptr [rax+0x128], r13
       jne      SHORT G_M000_IG45
       xor      ebx, ebx
       jmp      G_M000_IG18
 
G_M000_IG45:                ;; offset=0x0537
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       lea      rdi, bword ptr [rax+0x38]
       mov      rsi, gword ptr [rbp-0x128]
       call     CORINFO_HELP_ASSIGN_REF
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      dword ptr [rax+0x150], ebx
       jmp      G_M000_IG18
 
G_M000_IG46:                ;; offset=0x0566
       mov      rdi, gword ptr [rbp-0xA0]
       cmp      dword ptr [rdi+0x08], 0
       jbe      G_M000_IG38
       mov      r8, gword ptr [rbp-0xA0]
       add      r8, 16
       jmp      G_M000_IG21
 
G_M000_IG47:                ;; offset=0x0587
       mov      r15, gword ptr [rbp-0x130]
       cmp      ebx, 1
       je       SHORT G_M000_IG48
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, qword ptr [rax+0x28]
       mov      rsi, qword ptr [rax+0x18]
       lea      rdx, [rbp-0x30]
       lea      rcx, [rbp-0x88]
       mov      r8, gword ptr [rbp-0x128]
       mov      r9, r15
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinH_e(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       mov      r15, rax
       jmp      SHORT G_M000_IG49
 
G_M000_IG48:                ;; offset=0x05C2
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, qword ptr [rax+0x28]
       mov      rsi, qword ptr [rax+0x18]
       lea      rdx, [rbp-0x30]
       lea      rcx, [rbp-0x88]
       mov      r8, gword ptr [rbp-0x128]
       mov      r9, r15
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinR_e(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       mov      r15, rax
 
G_M000_IG49:                ;; offset=0x05EF
       jmp      G_M000_IG27
 
G_M000_IG50:                ;; offset=0x05F4
       mov      rdi, qword ptr [rax+0x18]
       lea      rsi, [rbp-0x90]
       lea      rdx, [rbp-0x98]
       mov      rcx, ADDR
       mov      qword ptr [rbp-0x160], rcx
       lea      rcx, G_M000_IG53
       mov      qword ptr [rbp-0x148], rcx
       lea      rcx, bword ptr [rbp-0x170]
       mov      qword ptr [r14+0x10], rcx
       mov      byte  ptr [r14+0x0C], 0
 
G_M000_IG51:                ;; offset=0x0635
       mov      rax, ADDR
 
G_M000_IG52:                ;; offset=0x063F
       call     rax ; Armonik.Ffi.Harness.Abi:ak_enc_take(long,ulong,ulong):int
 
G_M000_IG53:                ;; offset=0x0641
       mov      byte  ptr [r14+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG54
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG54:                ;; offset=0x0655
       mov      rcx, bword ptr [rbp-0x168]
       mov      qword ptr [r14+0x10], rcx
       mov      edi, eax
       test     edi, edi
       mov      rax, gword ptr [rbp-0x120]
       je       G_M000_IG29
       jmp      G_M000_IG63
 
G_M000_IG55:                ;; offset=0x0676
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
       jne      G_M000_IG30
 
G_M000_IG56:                ;; offset=0x06BE
       test     ebx, ebx
       jne      G_M000_IG39
       mov      r15, gword ptr [rbp-0x130]
       mov      gword ptr [rbp-0xA0], r15
       test     r15, r15
       jne      G_M000_IG37
 
G_M000_IG57:                ;; offset=0x06DD
       xor      r8d, r8d
 
G_M000_IG58:                ;; offset=0x06E0
       mov      rax, gword ptr [rbp-0x120]
       mov      rdi, qword ptr [rax+0x28]
       mov      rsi, qword ptr [rax+0x18]
       mov      r9d, dword ptr [r15+0x08]
       lea      rcx, [rbp-0x118]
       lea      rdx, [rbp-0x30]
       mov      r10, ADDR
       mov      qword ptr [rbp-0x160], r10
       lea      r10, G_M000_IG60
       mov      qword ptr [rbp-0x148], r10
       lea      r10, bword ptr [rbp-0x170]
       mov      qword ptr [r14+0x10], r10
       mov      byte  ptr [r14+0x0C], 0
 
G_M000_IG59:                ;; offset=0x072D
       call     [Armonik.Ffi.Harness.Abi:ak_uencode_UploadResultDataMessage(ulong,long,ulong,ulong,ulong,ulong):long]
 
G_M000_IG60:                ;; offset=0x0733
       mov      byte  ptr [r14+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG61
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG61:                ;; offset=0x0747
       mov      rdx, bword ptr [rbp-0x168]
       mov      qword ptr [r14+0x10], rdx
       mov      r15, rax
       jmp      G_M000_IG26
 
G_M000_IG62:                ;; offset=0x075A
       xor      r8, r8
       jmp      G_M000_IG15
 
G_M000_IG63:                ;; offset=0x0762
       xor      rcx, rcx
       mov      gword ptr [rbp-0xA0], rcx
 
G_M000_IG64:                ;; offset=0x076B
       mov      byte  ptr [rax+0x4C], 0
       mov      eax, edi
 
G_M000_IG65:                ;; offset=0x0771
       add      rsp, 344
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG66:                ;; offset=0x0783
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
 
G_M000_IG67:                ;; offset=0x079F
       mov      rax, gword ptr [rbp-0x120]
       mov      byte  ptr [rax+0x4C], 0
 
G_M000_IG68:                ;; offset=0x07AA
       add      rsp, 8
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
; Total bytes of code 1977

