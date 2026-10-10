; Assembly listing for method Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:Go(Armonik.Ffi.Facade.UploadResultDataMessage,bool,bool,byref,byref):int:this (Tier1)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1 code
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; partially interruptible
; with Dynamic PGO: edge weights are valid, and fgCalledCount is 8372
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
       mov      r14d, edx
       mov      r13d, ecx
       mov      r12, r8
 
G_M000_IG02:                ;; offset=0x007A
       lea      rdi, [rbp-0x200]
       mov      rsi, r10
       call     CORINFO_HELP_INIT_PINVOKE_FRAME
       mov      qword ptr [rbp-0x190], rax
       mov      rdi, rsp
       mov      qword ptr [rbp-0x1E0], rdi
       mov      rdi, rbp
       mov      qword ptr [rbp-0x1D0], rdi
       xor      edi, edi
       mov      bword ptr [rbp-0x1A8], r12
       mov      qword ptr [r12], rdi
 
G_M000_IG03:                ;; offset=0x00B1
       mov      rcx, bword ptr [rbp-0x1B0]
       mov      dword ptr [rcx], edi
       mov      gword ptr [rbp-0x198], rbx
       mov      rdi, qword ptr [rbx+0x18]
       mov      rdx, ADDR
       mov      qword ptr [rbp-0x1F0], rdx
       lea      rdx, G_M000_IG06
       mov      qword ptr [rbp-0x1D8], rdx
       lea      rdx, bword ptr [rbp-0x200]
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
       mov      rdi, bword ptr [rbp-0x1F8]
       mov      qword ptr [rbx+0x10], rdi
       mov      r15, gword ptr [rbp-0x198]
       mov      r12, gword ptr [r15+0x08]
       xor      edi, edi
       mov      dword ptr [r12+0x38], edi
       mov      rdi, gword ptr [r12+0x08]
       cmp      dword ptr [rdi+0x10], 0
       jbe      G_M000_IG64
       mov      rdi, gword ptr [rdi+0x08]
       cmp      dword ptr [rdi+0x08], 0
       jbe      G_M000_IG87
       mov      rdi, qword ptr [rdi+0x10]
       mov      qword ptr [r12+0x20], rdi
       mov      rdi, gword ptr [r12+0x10]
       cmp      dword ptr [rdi+0x10], 0
       jbe      G_M000_IG64
       mov      rdi, gword ptr [rdi+0x08]
       cmp      dword ptr [rdi+0x08], 0
       jbe      G_M000_IG87
       mov      edi, dword ptr [rdi+0x10]
       mov      dword ptr [r12+0x3C], edi
       xor      edi, edi
       mov      dword ptr [r12+0x40], edi
 
G_M000_IG08:                ;; offset=0x0188
       xor      eax, eax
 
G_M000_IG09:                ;; offset=0x018A
       mov      rdi, gword ptr [r12+0x18]
       cmp      eax, dword ptr [rdi+0x10]
       jl       G_M000_IG72
 
G_M000_IG10:                ;; offset=0x0198
       inc      dword ptr [rdi+0x14]
       xor      eax, eax
       mov      dword ptr [rdi+0x10], eax
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax+0x10], 3
       jl       G_M000_IG73
       mov      rdi, qword ptr [rax+0x18]
       mov      rdi, qword ptr [rdi+0x18]
       test     rdi, rdi
       je       G_M000_IG73
       mov      rax, bword ptr [rdi]
       add      rax, 16
       mov      rcx, gword ptr [rax+0x08]
       test     rcx, rcx
       jne      G_M000_IG74
 
G_M000_IG11:                ;; offset=0x01E5
       cmp      dword ptr [(reloc ADDR)], 7
       je       G_M000_IG75
 
G_M000_IG12:                ;; offset=0x01F2
       cmp      dword ptr [(reloc ADDR)], 6
       jne      G_M000_IG76
 
G_M000_IG13:                ;; offset=0x01FF
       mov      r12d, 1
 
G_M000_IG14:                ;; offset=0x0205
       xor      edx, edx
       xor      esi, esi
       test     r12d, r12d
       je       SHORT G_M000_IG17
 
G_M000_IG15:                ;; offset=0x020E
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax], 2
       jl       G_M000_IG77
       mov      rdx, qword ptr [rax+0x08]
       mov      rax, qword ptr [rdx+0x10]
       test     rax, rax
       je       G_M000_IG77
 
G_M000_IG16:                ;; offset=0x023E
       mov      rcx, rax
       mov      rdx, qword ptr [rcx+0x128]
       mov      qword ptr [rbp-0x30], rdx
       mov      rsi, qword ptr [rcx+0x130]
       mov      qword ptr [rbp-0x38], rsi
       mov      rdx, qword ptr [rbp-0x30]
       mov      rsi, qword ptr [rbp-0x38]
 
G_M000_IG17:                ;; offset=0x025F
       mov      rdi, qword ptr [r15+0x28]
       mov      eax, dword ptr [r15+0x40]
       mov      dword ptr [rdi], eax
       mov      rdi, qword ptr [r15+0x28]
       xor      eax, eax
       test     r14b, r14b
       setne    al
       mov      dword ptr [rdi+0x04], eax
       xor      edi, edi
       mov      qword ptr [rbp-0x40], rdi
       mov      rax, gword ptr [rbp-0x1A0]
       mov      rcx, gword ptr [rax+0x08]
       mov      gword ptr [rbp-0x1C0], rcx
       mov      rdi, rcx
       test     rdi, rdi
       je       G_M000_IG56
 
G_M000_IG18:                ;; offset=0x029C
       mov      r8, gword ptr [rdi+0x18]
 
G_M000_IG19:                ;; offset=0x02A0
       test     r8, r8
       je       G_M000_IG78
 
G_M000_IG20:                ;; offset=0x02A9
       mov      gword ptr [rbp-0x1B8], r8
       test     r14b, r14b
       jne      G_M000_IG46
 
G_M000_IG21:                ;; offset=0x02B9
       vxorps   ymm0, ymm0, ymm0
       vmovdqu32 zmmword ptr [rbp-0x128], zmm0
       vmovdqu  ymmword ptr [rbp-0xF0], ymm0
       mov      rbx, gword ptr [r15+0x08]
       vxorps   ymm0, ymm0, ymm0
       vmovdqu32 zmmword ptr [rbp-0x128], zmm0
       vmovdqu  ymmword ptr [rbp-0xF0], ymm0
       test     rcx, rcx
       je       G_M000_IG30
 
G_M000_IG22:                ;; offset=0x02F2
       mov      r14, rcx
       vxorps   ymm0, ymm0, ymm0
       vmovdqu32 zmmword ptr [rbp-0x128], zmm0
       vmovdqu  xmmword ptr [rbp-0xE8], xmm0
       mov      rcx, gword ptr [r14+0x08]
       cmp      byte  ptr [rbx], bl
       test     rcx, rcx
       je       SHORT G_M000_IG23
       cmp      dword ptr [rcx+0x08], 0
       jne      SHORT G_M000_IG24
 
G_M000_IG23:                ;; offset=0x031C
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbp-0x168], xmm0
       vmovdqu  xmmword ptr [rbp-0x160], xmm0
       jmp      SHORT G_M000_IG25
 
G_M000_IG24:                ;; offset=0x0332
       mov      gword ptr [rbp-0x1A0], rax
       mov      qword ptr [rbp-0x30], rdx
       mov      qword ptr [rbp-0x38], rsi
       lea      rsi, [rbp-0x168]
       mov      rdi, rbx
       mov      rdx, rcx
       call     [Armonik.Ffi.Harness.Stage:StrPresent(System.String):Armonik.Ffi.Harness.ak_str:this]
       mov      rax, gword ptr [rbp-0x1A0]
       mov      rdx, qword ptr [rbp-0x30]
       mov      rsi, qword ptr [rbp-0x38]
 
G_M000_IG25:                ;; offset=0x0363
       vmovdqu  xmm0, xmmword ptr [rbp-0x168]
       vmovdqu  xmmword ptr [rbp-0x128], xmm0
       mov      rdi, qword ptr [rbp-0x158]
       mov      qword ptr [rbp-0x118], rdi
       mov      rcx, gword ptr [r14+0x10]
       test     rcx, rcx
       je       SHORT G_M000_IG26
       cmp      dword ptr [rcx+0x08], 0
       jne      SHORT G_M000_IG27
 
G_M000_IG26:                ;; offset=0x0390
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbp-0x180], xmm0
       vmovdqu  xmmword ptr [rbp-0x178], xmm0
       jmp      SHORT G_M000_IG28
 
G_M000_IG27:                ;; offset=0x03A6
       mov      gword ptr [rbp-0x1A0], rax
       mov      qword ptr [rbp-0x30], rdx
       mov      qword ptr [rbp-0x38], rsi
       lea      rsi, [rbp-0x180]
       mov      rdi, rbx
       mov      rdx, rcx
       call     [Armonik.Ffi.Harness.Stage:StrPresent(System.String):Armonik.Ffi.Harness.ak_str:this]
       mov      rax, gword ptr [rbp-0x1A0]
       mov      rdx, qword ptr [rbp-0x30]
       mov      rsi, qword ptr [rbp-0x38]
 
G_M000_IG28:                ;; offset=0x03D7
       vmovdqu  xmm0, xmmword ptr [rbp-0x180]
       vmovdqu  xmmword ptr [rbp-0x110], xmm0
       mov      rdi, qword ptr [rbp-0x170]
       mov      qword ptr [rbp-0x100], rdi
       vxorps   xmm0, xmm0, xmm0
       vmovdqu  xmmword ptr [rbp-0x150], xmm0
       vmovdqu  xmmword ptr [rbp-0x148], xmm0
       mov      qword ptr [rbp-0x150], 1
       lea      rbx, bword ptr [rbp-0x128]
       lea      rcx, bword ptr [rbp-0x150]
       mov      rdi, gword ptr [r14+0x18]
       test     rdi, rdi
       je       G_M000_IG79
       mov      r14d, dword ptr [rdi+0x08]
 
G_M000_IG29:                ;; offset=0x0433
       movsxd   rdi, r14d
       mov      qword ptr [rcx+0x08], rdi
       xor      edi, edi
       mov      qword ptr [rbp-0x140], rdi
       vmovdqu  xmm0, xmmword ptr [rbp-0x150]
       vmovdqu  xmmword ptr [rbx+0x30], xmm0
       mov      rdi, qword ptr [rbp-0x140]
       mov      qword ptr [rbx+0x40], rdi
       mov      edi, dword ptr [rbp-0xD8]
       or       edi, 1
       mov      dword ptr [rbp-0xD8], edi
 
G_M000_IG30:                ;; offset=0x046A
       test     r13b, r13b
       je       G_M000_IG45
       inc      qword ptr [(reloc ADDR)]
       test     r12d, r12d
       je       G_M000_IG34
 
G_M000_IG31:                ;; offset=0x0483
       mov      gword ptr [rbp-0x1A0], rax
       mov      qword ptr [rbp-0x30], rdx
       mov      qword ptr [rbp-0x38], rsi
       mov      rdi, ADDR
       mov      rcx, ADDR
       call     rcx
       cmp      dword ptr [rax], 2
       jl       G_M000_IG80
       mov      rdi, qword ptr [rax+0x08]
       mov      rcx, qword ptr [rdi+0x10]
       test     rcx, rcx
       je       G_M000_IG80
       mov      r14, rcx
       mov      rbx, qword ptr [rbp-0x30]
       cmp      qword ptr [r14+0x128], rbx
       je       G_M000_IG81
 
G_M000_IG32:                ;; offset=0x04D6
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax+0x10], 2
       jl       G_M000_IG82
       mov      rdi, qword ptr [rax+0x18]
       mov      rdi, qword ptr [rdi+0x10]
       test     rdi, rdi
       je       G_M000_IG82
       mov      rax, bword ptr [rdi]
       add      rax, 16
 
G_M000_IG33:                ;; offset=0x050E
       lea      rdi, bword ptr [rax+0x38]
       mov      rsi, gword ptr [rbp-0x1A0]
       call     CORINFO_HELP_ASSIGN_REF
       mov      dword ptr [r14+0x150], r12d
       mov      rax, gword ptr [rbp-0x1A0]
       mov      rdx, qword ptr [rbp-0x30]
       mov      rsi, qword ptr [rbp-0x38]
 
G_M000_IG34:                ;; offset=0x0534
       test     r12d, r12d
       je       G_M000_IG83
       cmp      r12d, 1
       jne      G_M000_IG43
 
G_M000_IG35:                ;; offset=0x0547
       mov      qword ptr [rbp-0x30], rdx
       mov      qword ptr [rbp-0x38], rsi
       mov      rdi, qword ptr [r15+0x28]
       mov      rsi, qword ptr [r15+0x18]
       lea      rdx, [rbp-0x40]
       lea      rcx, [rbp-0x128]
       mov      r8, rax
       mov      r9, gword ptr [rbp-0x1B8]
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinR_e(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
 
G_M000_IG36:                ;; offset=0x0572
       mov      rbx, rax
 
G_M000_IG37:                ;; offset=0x0575
       mov      r13, gword ptr [r15+0x08]
       cmp      byte  ptr [r13], r13b
       xor      r14d, r14d
 
G_M000_IG38:                ;; offset=0x0580
       mov      rdi, gword ptr [r13+0x18]
       cmp      r14d, dword ptr [rdi+0x10]
       jl       G_M000_IG63
 
G_M000_IG39:                ;; offset=0x058E
       inc      dword ptr [rdi+0x14]
       xor      eax, eax
       mov      dword ptr [rdi+0x10], eax
       test     r12d, r12d
       je       G_M000_IG44
 
G_M000_IG40:                ;; offset=0x059F
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax], 2
       jl       SHORT G_M000_IG42
       mov      rdi, qword ptr [rax+0x08]
       mov      r14, qword ptr [rdi+0x10]
       test     r14, r14
       je       SHORT G_M000_IG42
 
G_M000_IG41:                ;; offset=0x05C7
       xor      edi, edi
       mov      dword ptr [r14+0x150], edi
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax+0x10], 2
       jl       G_M000_IG66
       mov      rdi, qword ptr [rax+0x18]
       mov      rdi, qword ptr [rdi+0x10]
       test     rdi, rdi
       je       G_M000_IG66
       mov      rax, bword ptr [rdi]
       add      rax, 16
       jmp      G_M000_IG54
 
G_M000_IG42:                ;; offset=0x060D
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      r14, rax
       jmp      SHORT G_M000_IG41
 
G_M000_IG43:                ;; offset=0x061C
       mov      qword ptr [rbp-0x30], rdx
       mov      qword ptr [rbp-0x38], rsi
       mov      rdi, qword ptr [r15+0x28]
       mov      rsi, qword ptr [r15+0x18]
       lea      rdx, [rbp-0x40]
       lea      rcx, [rbp-0x128]
       mov      r8, rax
       mov      r9, gword ptr [rbp-0x1B8]
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinH_e(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       jmp      G_M000_IG36
 
G_M000_IG44:                ;; offset=0x064C
       test     rbx, rbx
       jl       G_M000_IG68
       cmp      byte  ptr [r15+0x4C], 0
       je       G_M000_IG69
 
G_M000_IG45:                ;; offset=0x0660
       xor      eax, eax
       jmp      G_M000_IG71
 
G_M000_IG46:                ;; offset=0x0667
       mov      qword ptr [rbp-0x30], rdx
       mov      qword ptr [rbp-0x38], rsi
       vxorps   ymm2, ymm2, ymm2
       vmovdqu32 zmmword ptr [rbp-0xC8], zmm2
       vmovdqu32 zmmword ptr [rbp-0x90], zmm2
       mov      rdx, gword ptr [r15+0x08]
       lea      rdi, [rbp-0xC8]
       mov      gword ptr [rbp-0x1A0], rax
       mov      rsi, rax
       call     [Armonik.Ffi.Harness.G:U_UploadResultDataMessage(byref,Armonik.Ffi.Facade.UploadResultDataMessage,Armonik.Ffi.Harness.Stage)]
       test     r13b, r13b
       je       SHORT G_M000_IG45
       inc      qword ptr [(reloc ADDR)]
       test     r12d, r12d
       jne      G_M000_IG57
 
G_M000_IG47:                ;; offset=0x06B7
       test     r12d, r12d
       jne      G_M000_IG60
       mov      r14, gword ptr [rbp-0x1B8]
       mov      gword ptr [rbp-0xD0], r14
       test     r14, r14
       jne      G_M000_IG59
 
G_M000_IG48:                ;; offset=0x06D7
       xor      r8d, r8d
 
G_M000_IG49:                ;; offset=0x06DA
       mov      rdi, qword ptr [r15+0x28]
       mov      gword ptr [rbp-0x198], r15
       mov      rsi, qword ptr [r15+0x18]
       mov      r9d, dword ptr [r14+0x08]
       lea      rcx, [rbp-0xC8]
       lea      rdx, [rbp-0x40]
       mov      rax, ADDR
       mov      qword ptr [rbp-0x1F0], rax
       lea      rax, G_M000_IG51
       mov      qword ptr [rbp-0x1D8], rax
       lea      rax, bword ptr [rbp-0x200]
       mov      qword ptr [rbx+0x10], rax
       mov      byte  ptr [rbx+0x0C], 0
 
G_M000_IG50:                ;; offset=0x0726
       call     [Armonik.Ffi.Harness.Abi:ak_uencode_UploadResultDataMessage(ulong,long,ulong,ulong,ulong,ulong):long]
 
G_M000_IG51:                ;; offset=0x072C
       mov      byte  ptr [rbx+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG52
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG52:                ;; offset=0x073F
       mov      rsi, bword ptr [rbp-0x1F8]
       mov      qword ptr [rbx+0x10], rsi
       mov      r15, rax
 
G_M000_IG53:                ;; offset=0x074D
       xor      rsi, rsi
       mov      gword ptr [rbp-0xD0], rsi
       mov      rbx, r15
       mov      r15, gword ptr [rbp-0x198]
       jmp      G_M000_IG37
 
G_M000_IG54:                ;; offset=0x0765
       xor      rdi, rdi
       mov      gword ptr [rax+0x38], rdi
       mov      rdi, qword ptr [r14+0x128]
       sub      rdi, qword ptr [rbp-0x30]
       mov      rax, qword ptr [r14+0x130]
       sub      rax, qword ptr [rbp-0x38]
       cmp      rdi, rax
       jne      G_M000_IG67
 
G_M000_IG55:                ;; offset=0x078A
       jmp      G_M000_IG44
 
G_M000_IG56:                ;; offset=0x078F
       xor      r8, r8
       jmp      G_M000_IG19
 
G_M000_IG57:                ;; offset=0x0797
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      r14, rax
       mov      r13, qword ptr [rbp-0x30]
       cmp      qword ptr [r14+0x128], r13
       jne      SHORT G_M000_IG58
       xor      r12d, r12d
       jmp      G_M000_IG47
 
G_M000_IG58:                ;; offset=0x07B9
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       lea      rdi, bword ptr [rax+0x38]
       mov      rsi, gword ptr [rbp-0x1A0]
       call     CORINFO_HELP_ASSIGN_REF
       mov      dword ptr [r14+0x150], r12d
       jmp      G_M000_IG47
 
G_M000_IG59:                ;; offset=0x07DF
       mov      rdi, gword ptr [rbp-0xD0]
       cmp      dword ptr [rdi+0x08], 0
       je       G_M000_IG48
       mov      rdi, gword ptr [rbp-0xD0]
       cmp      dword ptr [rdi+0x08], 0
       jbe      G_M000_IG87
       mov      r8, gword ptr [rbp-0xD0]
       add      r8, 16
       jmp      G_M000_IG49
 
G_M000_IG60:                ;; offset=0x0811
       mov      r14, gword ptr [rbp-0x1B8]
       cmp      r12d, 1
       je       SHORT G_M000_IG61
       mov      rdi, qword ptr [r15+0x28]
       mov      rsi, qword ptr [r15+0x18]
       lea      rdx, [rbp-0x40]
       lea      rcx, [rbp-0xC8]
       mov      r8, gword ptr [rbp-0x1A0]
       mov      r9, r14
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinH_u(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       mov      rbx, rax
       jmp      SHORT G_M000_IG62
 
G_M000_IG61:                ;; offset=0x0846
       mov      rdi, qword ptr [r15+0x28]
       mov      rsi, qword ptr [r15+0x18]
       lea      rdx, [rbp-0x40]
       lea      rcx, [rbp-0xC8]
       mov      r8, gword ptr [rbp-0x1A0]
       mov      r9, r14
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinR_u(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       mov      rbx, rax
 
G_M000_IG62:                ;; offset=0x086C
       jmp      G_M000_IG37
 
G_M000_IG63:                ;; offset=0x0871
       mov      rax, rdi
       cmp      r14d, dword ptr [rax+0x10]
       jb       SHORT G_M000_IG65
 
G_M000_IG64:                ;; offset=0x087A
       call     [System.ThrowHelper:ThrowArgumentOutOfRange_IndexMustBeLessException()]
       int3     
 
G_M000_IG65:                ;; offset=0x0881
       mov      rdi, rax
       mov      rdi, gword ptr [rdi+0x08]
       cmp      r14d, dword ptr [rdi+0x08]
       jae      G_M000_IG87
       mov      edx, r14d
       mov      rdi, qword ptr [rdi+8*rdx+0x10]
       mov      qword ptr [rbp-0x188], rdi
       lea      rdi, [rbp-0x188]
       call     [System.Runtime.InteropServices.GCHandle:Free():this]
       inc      r14d
       jmp      G_M000_IG38
 
G_M000_IG66:                ;; offset=0x08B6
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG54
 
G_M000_IG67:                ;; offset=0x08C5
       mov      rdi, -1
       test     rbx, rbx
       cmovge   rbx, rdi
       jmp      G_M000_IG44
 
G_M000_IG68:                ;; offset=0x08DB
       mov      eax, ebx
       jmp      SHORT G_M000_IG71
 
G_M000_IG69:                ;; offset=0x08DF
       mov      rdi, qword ptr [r15+0x18]
       lea      rsi, [rbp-0x48]
       lea      rdx, [rbp-0x50]
       call     Armonik.Ffi.Harness.Abi:ak_enc_take(long,ulong,ulong):int
       test     eax, eax
       je       SHORT G_M000_IG70
       jmp      SHORT G_M000_IG71
 
G_M000_IG70:                ;; offset=0x08F6
       mov      rax, qword ptr [rbp-0x48]
       mov      r12, bword ptr [rbp-0x1A8]
       mov      qword ptr [r12], rax
       mov      eax, dword ptr [rbp-0x50]
       mov      r14, bword ptr [rbp-0x1B0]
       mov      dword ptr [r14], eax
       jmp      G_M000_IG45
 
G_M000_IG71:                ;; offset=0x0917
       add      rsp, 488
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG72:                ;; offset=0x0929
       cmp      eax, dword ptr [rdi+0x10]
       jae      G_M000_IG64
       mov      rdi, gword ptr [rdi+0x08]
       cmp      eax, dword ptr [rdi+0x08]
       jae      G_M000_IG87
       mov      dword ptr [rbp-0x12C], eax
       mov      ecx, eax
       mov      rdi, qword ptr [rdi+8*rcx+0x10]
       mov      qword ptr [rbp-0x138], rdi
       lea      rdi, [rbp-0x138]
       call     [System.Runtime.InteropServices.GCHandle:Free():this]
       mov      eax, dword ptr [rbp-0x12C]
       inc      eax
       jmp      G_M000_IG09
 
G_M000_IG73:                ;; offset=0x096D
       mov      edi, 3
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      rcx, gword ptr [rax+0x08]
       test     rcx, rcx
       je       G_M000_IG11
 
G_M000_IG74:                ;; offset=0x0984
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
 
G_M000_IG75:                ;; offset=0x09B0
       xor      edi, edi
       call     [Armonik.Ffi.Harness.Stage:ReleaseChunk(int)]
       cmp      dword ptr [(reloc ADDR)], 6
       je       G_M000_IG13
 
G_M000_IG76:                ;; offset=0x09C5
       mov      r12d, 2
       xor      edi, edi
       cmp      dword ptr [(reloc ADDR)], 7
       cmovne   r12d, edi
       jmp      G_M000_IG14
 
G_M000_IG77:                ;; offset=0x09DD
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG16
 
G_M000_IG78:                ;; offset=0x09EC
       mov      gword ptr [rbp-0x1A0], rax
       mov      qword ptr [rbp-0x30], rdx
       mov      qword ptr [rbp-0x38], rsi
       mov      rdi, ADDR
       mov      esi, 11
       call     CORINFO_HELP_CLASSINIT_SHARED_DYNAMICCLASS
       mov      rdi, ADDR
       mov      r8, gword ptr [rdi]
       mov      rdi, r8
       mov      r8, rdi
       mov      rax, gword ptr [rbp-0x1A0]
       mov      rcx, gword ptr [rbp-0x1C0]
       mov      rdx, qword ptr [rbp-0x30]
       mov      rsi, qword ptr [rbp-0x38]
       jmp      G_M000_IG20
 
G_M000_IG79:                ;; offset=0x0A3D
       xor      r14d, r14d
       jmp      G_M000_IG29
 
G_M000_IG80:                ;; offset=0x0A45
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      rcx, rax
       mov      r14, rcx
       mov      rbx, qword ptr [rbp-0x30]
       cmp      qword ptr [r14+0x128], rbx
       jne      G_M000_IG32
 
G_M000_IG81:                ;; offset=0x0A66
       xor      r12d, r12d
       mov      rax, gword ptr [rbp-0x1A0]
       mov      rdx, qword ptr [rbp-0x30]
       mov      rsi, qword ptr [rbp-0x38]
       jmp      G_M000_IG34
 
G_M000_IG82:                ;; offset=0x0A7D
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       jmp      G_M000_IG33
 
G_M000_IG83:                ;; offset=0x0A8C
       mov      r9, gword ptr [rbp-0x1B8]
       mov      gword ptr [rbp-0xD0], r9
       test     r9, r9
       je       SHORT G_M000_IG84
       mov      rdi, gword ptr [rbp-0xD0]
       cmp      dword ptr [rdi+0x08], 0
       jne      SHORT G_M000_IG85
 
G_M000_IG84:                ;; offset=0x0AAC
       xor      r8d, r8d
       mov      qword ptr [rbp-0x30], rdx
       mov      qword ptr [rbp-0x38], rsi
       jmp      SHORT G_M000_IG86
 
G_M000_IG85:                ;; offset=0x0AB9
       mov      r8, gword ptr [rbp-0xD0]
       cmp      dword ptr [r8+0x08], 0
       jbe      SHORT G_M000_IG87
       mov      r8, gword ptr [rbp-0xD0]
       add      r8, 16
       mov      qword ptr [rbp-0x30], rdx
       mov      qword ptr [rbp-0x38], rsi
 
G_M000_IG86:                ;; offset=0x0ADA
       mov      rdi, qword ptr [r15+0x28]
       mov      rsi, qword ptr [r15+0x18]
       mov      r9d, dword ptr [r9+0x08]
       lea      rcx, [rbp-0x128]
       lea      rdx, [rbp-0x40]
       call     Armonik.Ffi.Harness.Abi:ak_encode_UploadResultDataMessage(ulong,long,ulong,ulong,ulong,ulong):long
       mov      rbx, rax
       mov      gword ptr [rbp-0x198], r15
       mov      r15, rbx
       jmp      G_M000_IG53
 
G_M000_IG87:                ;; offset=0x0B08
       call     CORINFO_HELP_RNGCHKFAIL
       int3     
 
; Total bytes of code 2830

