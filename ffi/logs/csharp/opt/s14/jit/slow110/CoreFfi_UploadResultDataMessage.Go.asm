; Assembly listing for method Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:Go(Armonik.Ffi.Facade.UploadResultDataMessage,bool,bool,byref,byref):int:this (Tier1)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1 code
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; partially interruptible
; with Dynamic PGO: edge weights are invalid, and fgCalledCount is 23732
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
       jbe      G_M000_IG59
 
G_M000_IG08:                ;; offset=0x012C
       mov      rdi, gword ptr [rdi+0x08]
       cmp      dword ptr [rdi+0x08], 0
       jbe      G_M000_IG82
       mov      rdi, qword ptr [rdi+0x10]
       mov      qword ptr [r12+0x20], rdi
       mov      rdi, gword ptr [r12+0x10]
       cmp      dword ptr [rdi+0x10], 0
       jbe      G_M000_IG59
       mov      rdi, gword ptr [rdi+0x08]
       cmp      dword ptr [rdi+0x08], 0
       jbe      G_M000_IG82
       mov      edi, dword ptr [rdi+0x10]
       mov      dword ptr [r12+0x3C], edi
       xor      edi, edi
       mov      dword ptr [r12+0x40], edi
 
G_M000_IG09:                ;; offset=0x016F
       xor      eax, eax
 
G_M000_IG10:                ;; offset=0x0171
       mov      rdi, gword ptr [r12+0x18]
       cmp      eax, dword ptr [rdi+0x10]
       jl       G_M000_IG66
 
G_M000_IG11:                ;; offset=0x017F
       inc      dword ptr [rdi+0x14]
       xor      eax, eax
       mov      dword ptr [rdi+0x10], eax
       mov      rdi, ADDR
       mov      rax, ADDR
       call     rax
       cmp      dword ptr [rax+0x10], 3
       jl       G_M000_IG67
       mov      rdx, qword ptr [rax+0x18]
       mov      rdx, qword ptr [rdx+0x18]
       test     rdx, rdx
       je       G_M000_IG67
       mov      rax, bword ptr [rdx]
       add      rax, 16
       mov      rcx, gword ptr [rax+0x08]
       test     rcx, rcx
       jne      G_M000_IG68
 
G_M000_IG12:                ;; offset=0x01CC
       cmp      dword ptr [(reloc ADDR)], 7
       je       G_M000_IG69
 
G_M000_IG13:                ;; offset=0x01D9
       mov      edx, dword ptr [(reloc ADDR)]
       mov      r12d, 1
       mov      edi, 2
       xor      esi, esi
       cmp      edx, 7
       cmovne   edi, esi
       cmp      edx, 6
       cmovne   r12d, edi
 
G_M000_IG14:                ;; offset=0x01F9
       mov      qword ptr [rbp-0x30], rsi
 
G_M000_IG15:                ;; offset=0x01FD
       mov      qword ptr [rbp-0x38], rsi
       test     r12d, r12d
       jne      G_M000_IG70
 
G_M000_IG16:                ;; offset=0x020A
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
       mov      r10, gword ptr [rbp-0x1A8]
       mov      rax, gword ptr [r10+0x08]
       mov      gword ptr [rbp-0x1D0], rax
       mov      rdx, rax
       test     rdx, rdx
       je       G_M000_IG47
 
G_M000_IG17:                ;; offset=0x0248
       mov      r11, gword ptr [rdx+0x18]
 
G_M000_IG18:                ;; offset=0x024C
       test     r11, r11
       je       G_M000_IG71
 
G_M000_IG19:                ;; offset=0x0255
       mov      gword ptr [rbp-0x1C0], r11
       test     r15b, r15b
       jne      G_M000_IG48
 
G_M000_IG20:                ;; offset=0x0265
       vxorps   ymm2, ymm2, ymm2
       vmovdqu32 zmmword ptr [rbp-0x128], zmm2
       vmovdqu  ymmword ptr [rbp-0xF0], ymm2
       mov      r15, gword ptr [r14+0x08]
       vxorps   ymm2, ymm2, ymm2
       vmovdqu32 zmmword ptr [rbp-0x128], zmm2
       vmovdqu  ymmword ptr [rbp-0xF0], ymm2
       test     rax, rax
       je       G_M000_IG29
 
G_M000_IG21:                ;; offset=0x029E
       mov      gword ptr [rbp-0x1C8], rax
       vxorps   ymm2, ymm2, ymm2
       vmovdqu32 zmmword ptr [rbp-0x128], zmm2
       vmovdqu  xmmword ptr [rbp-0xE8], xmm2
       mov      rdx, gword ptr [rax+0x08]
       cmp      byte  ptr [r15], r15b
       test     rdx, rdx
       je       SHORT G_M000_IG23
 
G_M000_IG22:                ;; offset=0x02C7
       cmp      dword ptr [rdx+0x08], 0
       jne      G_M000_IG45
 
G_M000_IG23:                ;; offset=0x02D1
       vxorps   xmm2, xmm2, xmm2
       vmovdqu  xmmword ptr [rbp-0x170], xmm2
       vmovdqu  xmmword ptr [rbp-0x168], xmm2
 
G_M000_IG24:                ;; offset=0x02E5
       vmovdqu  xmm2, xmmword ptr [rbp-0x170]
       vmovdqu  xmmword ptr [rbp-0x128], xmm2
       mov      rdi, qword ptr [rbp-0x160]
       mov      qword ptr [rbp-0x118], rdi
       mov      rax, gword ptr [rbp-0x1C8]
       mov      rdx, gword ptr [rax+0x10]
       test     rdx, rdx
       je       SHORT G_M000_IG26
 
G_M000_IG25:                ;; offset=0x0313
       cmp      dword ptr [rdx+0x08], 0
       jne      G_M000_IG46
 
G_M000_IG26:                ;; offset=0x031D
       vxorps   xmm2, xmm2, xmm2
       vmovdqu  xmmword ptr [rbp-0x188], xmm2
       vmovdqu  xmmword ptr [rbp-0x180], xmm2
 
G_M000_IG27:                ;; offset=0x0331
       vmovdqu  xmm2, xmmword ptr [rbp-0x188]
       vmovdqu  xmmword ptr [rbp-0x110], xmm2
       mov      rdi, qword ptr [rbp-0x178]
       mov      qword ptr [rbp-0x100], rdi
       vxorps   xmm2, xmm2, xmm2
       vmovdqu  xmmword ptr [rbp-0x158], xmm2
       vmovdqu  xmmword ptr [rbp-0x150], xmm2
       mov      qword ptr [rbp-0x158], 1
       lea      r15, bword ptr [rbp-0x128]
       lea      r11, bword ptr [rbp-0x158]
       mov      rax, gword ptr [rbp-0x1C8]
       mov      rdi, gword ptr [rax+0x18]
       test     rdi, rdi
       je       G_M000_IG72
       mov      eax, dword ptr [rdi+0x08]
 
G_M000_IG28:                ;; offset=0x0393
       movsxd   rdi, eax
       mov      qword ptr [r11+0x08], rdi
       xor      edi, edi
       mov      qword ptr [rbp-0x148], rdi
       vmovdqu  xmm2, xmmword ptr [rbp-0x158]
       vmovdqu  xmmword ptr [r15+0x30], xmm2
       mov      rdi, qword ptr [rbp-0x148]
       mov      qword ptr [r15+0x40], rdi
       mov      edi, dword ptr [rbp-0xD8]
       or       edi, 1
       mov      dword ptr [rbp-0xD8], edi
 
G_M000_IG29:                ;; offset=0x03CB
       test     r13b, r13b
       je       G_M000_IG78
       mov      r15, ADDR
       inc      qword ptr [r15]
       test     r12d, r12d
       jne      G_M000_IG44
 
G_M000_IG30:                ;; offset=0x03EA
       test     r12d, r12d
       jne      G_M000_IG75
       mov      rsi, gword ptr [rbp-0x1C0]
       mov      gword ptr [rbp-0xD0], rsi
       test     rsi, rsi
       je       SHORT G_M000_IG32
 
G_M000_IG31:                ;; offset=0x0406
       mov      rdi, gword ptr [rbp-0xD0]
       cmp      dword ptr [rdi+0x08], 0
       jne      G_M000_IG74
 
G_M000_IG32:                ;; offset=0x0417
       xor      edx, edx
       mov      qword ptr [rbp-0x130], rdx
 
G_M000_IG33:                ;; offset=0x0420
       mov      rdi, qword ptr [r14+0x28]
       mov      gword ptr [rbp-0x1A0], r14
       mov      rsi, qword ptr [r14+0x18]
       mov      r15, gword ptr [rbp-0x1C0]
       mov      r9d, dword ptr [r15+0x08]
       lea      rcx, [rbp-0x128]
       lea      rdx, [rbp-0x40]
       mov      r8, qword ptr [rbp-0x130]
       mov      rax, ADDR
       mov      qword ptr [rbp-0x200], rax
       lea      rax, G_M000_IG36
       mov      qword ptr [rbp-0x1E8], rax
       lea      rax, bword ptr [rbp-0x210]
       mov      qword ptr [rbx+0x10], rax
       mov      byte  ptr [rbx+0x0C], 0
 
G_M000_IG34:                ;; offset=0x047A
       mov      rax, ADDR
 
G_M000_IG35:                ;; offset=0x0484
       call     rax ; Armonik.Ffi.Harness.Abi:ak_encode_UploadResultDataMessage(ulong,long,ulong,ulong,ulong,ulong):long
 
G_M000_IG36:                ;; offset=0x0486
       mov      byte  ptr [rbx+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG37
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG37:                ;; offset=0x0499
       mov      rdi, bword ptr [rbp-0x208]
       mov      qword ptr [rbx+0x10], rdi
       mov      r14, rax
 
G_M000_IG38:                ;; offset=0x04A7
       xor      rsi, rsi
       mov      gword ptr [rbp-0xD0], rsi
 
G_M000_IG39:                ;; offset=0x04B0
       mov      r15, gword ptr [rbp-0x1A0]
       mov      rbx, gword ptr [r15+0x08]
       cmp      byte  ptr [rbx], bl
       xor      r13d, r13d
 
G_M000_IG40:                ;; offset=0x04C0
       mov      rdi, gword ptr [rbx+0x18]
       cmp      r13d, dword ptr [rdi+0x10]
       jl       G_M000_IG58
 
G_M000_IG41:                ;; offset=0x04CE
       inc      dword ptr [rdi+0x14]
       xor      eax, eax
       mov      dword ptr [rdi+0x10], eax
       test     r12d, r12d
       jne      G_M000_IG61
 
G_M000_IG42:                ;; offset=0x04DF
       test     r14, r14
       jl       G_M000_IG62
       cmp      byte  ptr [r15+0x4C], 0
       jne      G_M000_IG78
 
G_M000_IG43:                ;; offset=0x04F3
       mov      r14, r15
       jmp      G_M000_IG63
 
G_M000_IG44:                ;; offset=0x04FB
       mov      gword ptr [rbp-0x1A8], r10
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      r15, qword ptr [rbp-0x30]
       cmp      qword ptr [rax+0x128], r15
       jne      G_M000_IG73
       xor      r12d, r12d
       mov      r10, gword ptr [rbp-0x1A8]
       jmp      G_M000_IG30
 
G_M000_IG45:                ;; offset=0x052C
       mov      gword ptr [rbp-0x1A8], r10
       lea      rsi, [rbp-0x170]
       mov      rdi, r15
       call     [Armonik.Ffi.Harness.Stage:StrPresent(System.String):Armonik.Ffi.Harness.ak_str:this]
       mov      r10, gword ptr [rbp-0x1A8]
       jmp      G_M000_IG24
 
G_M000_IG46:                ;; offset=0x054F
       mov      gword ptr [rbp-0x1A8], r10
       lea      rsi, [rbp-0x188]
       mov      rdi, r15
       call     [Armonik.Ffi.Harness.Stage:StrPresent(System.String):Armonik.Ffi.Harness.ak_str:this]
       mov      r10, gword ptr [rbp-0x1A8]
       jmp      G_M000_IG27
 
G_M000_IG47:                ;; offset=0x0572
       xor      r11, r11
       jmp      G_M000_IG18
 
G_M000_IG48:                ;; offset=0x057A
       vxorps   ymm2, ymm2, ymm2
       vmovdqu32 zmmword ptr [rbp-0xC8], zmm2
       vmovdqu32 zmmword ptr [rbp-0x90], zmm2
       mov      rdx, gword ptr [r14+0x08]
       lea      rdi, [rbp-0xC8]
       mov      gword ptr [rbp-0x1A8], r10
       mov      rsi, r10
       call     [Armonik.Ffi.Harness.G:U_UploadResultDataMessage(byref,Armonik.Ffi.Facade.UploadResultDataMessage,Armonik.Ffi.Harness.Stage)]
       test     r13b, r13b
       je       G_M000_IG78
       mov      r15, ADDR
       inc      qword ptr [r15]
       test     r12d, r12d
       jne      G_M000_IG79
 
G_M000_IG49:                ;; offset=0x05CC
       test     r12d, r12d
       jne      G_M000_IG55
       mov      r15, gword ptr [rbp-0x1C0]
       mov      gword ptr [rbp-0xD0], r15
       test     r15, r15
       jne      G_M000_IG81
 
G_M000_IG50:                ;; offset=0x05EC
       xor      r8d, r8d
 
G_M000_IG51:                ;; offset=0x05EF
       mov      rdi, qword ptr [r14+0x28]
       mov      gword ptr [rbp-0x1A0], r14
       mov      rsi, qword ptr [r14+0x18]
       mov      r9d, dword ptr [r15+0x08]
       lea      rcx, [rbp-0xC8]
       lea      rdx, [rbp-0x40]
       mov      rax, ADDR
       mov      qword ptr [rbp-0x200], rax
       lea      rax, G_M000_IG53
       mov      qword ptr [rbp-0x1E8], rax
       lea      rax, bword ptr [rbp-0x210]
       mov      qword ptr [rbx+0x10], rax
       mov      byte  ptr [rbx+0x0C], 0
 
G_M000_IG52:                ;; offset=0x063B
       call     [Armonik.Ffi.Harness.Abi:ak_uencode_UploadResultDataMessage(ulong,long,ulong,ulong,ulong,ulong):long]
 
G_M000_IG53:                ;; offset=0x0641
       mov      byte  ptr [rbx+0x0C], 1
       cmp      dword ptr [(reloc ADDR)], 0
       je       SHORT G_M000_IG54
       call     [CORINFO_HELP_STOP_FOR_GC]
 
G_M000_IG54:                ;; offset=0x0654
       mov      rsi, bword ptr [rbp-0x208]
       mov      qword ptr [rbx+0x10], rsi
       mov      r14, rax
       jmp      G_M000_IG38
 
G_M000_IG55:                ;; offset=0x0667
       mov      r15, gword ptr [rbp-0x1C0]
       cmp      r12d, 1
       je       SHORT G_M000_IG56
       mov      rdi, qword ptr [r14+0x28]
       mov      rsi, qword ptr [r14+0x18]
       lea      rdx, [rbp-0x40]
       lea      rcx, [rbp-0xC8]
       mov      r8, gword ptr [rbp-0x1A8]
       mov      r9, r15
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinH_u(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       jmp      SHORT G_M000_IG57
 
G_M000_IG56:                ;; offset=0x0699
       mov      rdi, qword ptr [r14+0x28]
       mov      rsi, qword ptr [r14+0x18]
       lea      rdx, [rbp-0x40]
       lea      rcx, [rbp-0xC8]
       mov      r8, gword ptr [rbp-0x1A8]
       mov      r9, r15
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinR_u(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
 
G_M000_IG57:                ;; offset=0x06BC
       mov      r15, rax
       mov      gword ptr [rbp-0x1A0], r14
       mov      r14, r15
       jmp      G_M000_IG39
 
G_M000_IG58:                ;; offset=0x06CE
       mov      rax, rdi
       cmp      r13d, dword ptr [rax+0x10]
       jb       SHORT G_M000_IG60
 
G_M000_IG59:                ;; offset=0x06D7
       call     [System.ThrowHelper:ThrowArgumentOutOfRange_IndexMustBeLessException()]
       int3     
 
G_M000_IG60:                ;; offset=0x06DE
       mov      rdi, rax
       mov      rcx, r14
       mov      r14, r15
       mov      r15, rcx
       mov      rdi, gword ptr [rdi+0x08]
       cmp      r13d, dword ptr [rdi+0x08]
       jae      G_M000_IG82
       mov      edx, r13d
       mov      rdi, qword ptr [rdi+8*rdx+0x10]
       mov      qword ptr [rbp-0x190], rdi
       lea      rdi, [rbp-0x190]
       call     [System.Runtime.InteropServices.GCHandle:Free():this]
       inc      r13d
       mov      rax, r14
       mov      r14, r15
       mov      r15, rax
       jmp      G_M000_IG40
 
G_M000_IG61:                ;; offset=0x0725
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
       sub      rbx, qword ptr [rbp-0x30]
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      rax, qword ptr [rax+0x130]
       sub      rax, qword ptr [rbp-0x38]
       cmp      rbx, rax
       je       G_M000_IG42
       mov      rax, -1
       test     r14, r14
       cmovge   r14, rax
       jmp      G_M000_IG42
 
G_M000_IG62:                ;; offset=0x0790
       mov      eax, r14d
       jmp      SHORT G_M000_IG65
 
G_M000_IG63:                ;; offset=0x0795
       mov      rdi, qword ptr [r14+0x18]
       lea      rsi, [rbp-0x48]
       lea      rdx, [rbp-0x50]
       call     Armonik.Ffi.Harness.Abi:ak_enc_take(long,ulong,ulong):int
       test     eax, eax
       je       SHORT G_M000_IG64
       jmp      SHORT G_M000_IG65
 
G_M000_IG64:                ;; offset=0x07AC
       mov      rax, qword ptr [rbp-0x48]
       mov      r12, bword ptr [rbp-0x1B0]
       mov      qword ptr [r12], rax
       mov      eax, dword ptr [rbp-0x50]
       mov      rbx, bword ptr [rbp-0x1B8]
       mov      dword ptr [rbx], eax
       jmp      G_M000_IG78
 
G_M000_IG65:                ;; offset=0x07CC
       add      rsp, 504
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG66:                ;; offset=0x07DE
       cmp      eax, dword ptr [rdi+0x10]
       jae      G_M000_IG59
       mov      rdi, gword ptr [rdi+0x08]
       cmp      eax, dword ptr [rdi+0x08]
       jae      G_M000_IG82
       mov      dword ptr [rbp-0x134], eax
       mov      ecx, eax
       mov      rdi, qword ptr [rdi+8*rcx+0x10]
       mov      qword ptr [rbp-0x140], rdi
       lea      rdi, [rbp-0x140]
       call     [System.Runtime.InteropServices.GCHandle:Free():this]
       mov      eax, dword ptr [rbp-0x134]
       inc      eax
       jmp      G_M000_IG10
 
G_M000_IG67:                ;; offset=0x0822
       mov      edi, 3
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      rcx, gword ptr [rax+0x08]
       test     rcx, rcx
       je       G_M000_IG12
 
G_M000_IG68:                ;; offset=0x0839
       inc      dword ptr [rcx+0x14]
       mov      edx, dword ptr [rcx+0x10]
       xor      edi, edi
       mov      dword ptr [rcx+0x10], edi
       test     edx, edx
       jle      G_M000_IG12
       mov      rdi, gword ptr [rcx+0x08]
       xor      esi, esi
       call     [System.Array:Clear(System.Array,int,int)]
       cmp      dword ptr [(reloc ADDR)], 7
       jne      G_M000_IG13
 
G_M000_IG69:                ;; offset=0x0865
       xor      edi, edi
       call     [Armonik.Ffi.Harness.Stage:ReleaseChunk(int)]
       jmp      G_M000_IG13
 
G_M000_IG70:                ;; offset=0x0872
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      r8, qword ptr [rax+0x128]
       mov      qword ptr [rbp-0x30], r8
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      r9, qword ptr [rax+0x130]
       mov      qword ptr [rbp-0x38], r9
       jmp      G_M000_IG16
 
G_M000_IG71:                ;; offset=0x08A1
       mov      gword ptr [rbp-0x1A8], r10
       mov      rdi, ADDR
       mov      esi, 11
       call     CORINFO_HELP_CLASSINIT_SHARED_DYNAMICCLASS
       mov      rdi, ADDR
       mov      r11, gword ptr [rdi]
       mov      rax, gword ptr [rbp-0x1D0]
       mov      r10, gword ptr [rbp-0x1A8]
       jmp      G_M000_IG19
 
G_M000_IG72:                ;; offset=0x08DC
       xor      eax, eax
       jmp      G_M000_IG28
 
G_M000_IG73:                ;; offset=0x08E3
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       lea      rdi, bword ptr [rax+0x38]
       mov      rsi, gword ptr [rbp-0x1A8]
       call     CORINFO_HELP_ASSIGN_REF
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      dword ptr [rax+0x150], r12d
       mov      r10, gword ptr [rbp-0x1A8]
       jmp      G_M000_IG30
 
G_M000_IG74:                ;; offset=0x091A
       mov      rdi, gword ptr [rbp-0xD0]
       cmp      dword ptr [rdi+0x08], 0
       jbe      G_M000_IG82
       mov      rdi, gword ptr [rbp-0xD0]
       add      rdi, 16
       mov      rdx, rdi
       mov      qword ptr [rbp-0x130], rdx
       jmp      G_M000_IG33
 
G_M000_IG75:                ;; offset=0x0945
       cmp      r12d, 1
       je       SHORT G_M000_IG76
       mov      rdi, qword ptr [r14+0x28]
       mov      rsi, qword ptr [r14+0x18]
       lea      rdx, [rbp-0x40]
       lea      rcx, [rbp-0x128]
       mov      r8, r10
       mov      r9, gword ptr [rbp-0x1C0]
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinH_e(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       mov      r15, rax
       jmp      SHORT G_M000_IG77
 
G_M000_IG76:                ;; offset=0x0973
       mov      rdi, qword ptr [r14+0x28]
       mov      rsi, qword ptr [r14+0x18]
       lea      rdx, [rbp-0x40]
       lea      rcx, [rbp-0x128]
       mov      r8, r10
       mov      r9, gword ptr [rbp-0x1C0]
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:RootPinR_e(ulong,long,ulong,ulong,Armonik.Ffi.Facade.UploadResultDataMessage,ubyte[]):long]
       mov      r15, rax
 
G_M000_IG77:                ;; offset=0x0999
       mov      gword ptr [rbp-0x1A0], r14
       mov      r14, r15
       jmp      G_M000_IG39
 
G_M000_IG78:                ;; offset=0x09A8
       xor      eax, eax
       jmp      G_M000_IG65
 
G_M000_IG79:                ;; offset=0x09AF
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      r15, qword ptr [rbp-0x30]
       cmp      qword ptr [rax+0x128], r15
       jne      SHORT G_M000_IG80
       xor      r12d, r12d
       jmp      G_M000_IG49
 
G_M000_IG80:                ;; offset=0x09CE
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_GCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       lea      rdi, bword ptr [rax+0x38]
       mov      rsi, gword ptr [rbp-0x1A8]
       call     CORINFO_HELP_ASSIGN_REF
       mov      edi, 2
       call     CORINFO_HELP_GETSHARED_NONGCTHREADSTATIC_BASE_NOCTOR_OPTIMIZED
       mov      dword ptr [rax+0x150], r12d
       jmp      G_M000_IG49
 
G_M000_IG81:                ;; offset=0x09FE
       mov      rdi, gword ptr [rbp-0xD0]
       cmp      dword ptr [rdi+0x08], 0
       je       G_M000_IG50
       mov      rdi, gword ptr [rbp-0xD0]
       cmp      dword ptr [rdi+0x08], 0
       jbe      SHORT G_M000_IG82
       mov      r8, gword ptr [rbp-0xD0]
       add      r8, 16
       jmp      G_M000_IG51
 
G_M000_IG82:                ;; offset=0x0A2C
       call     CORINFO_HELP_RNGCHKFAIL
       int3     
 
; Total bytes of code 2610

