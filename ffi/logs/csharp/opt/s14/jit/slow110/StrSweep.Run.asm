; Assembly listing for method Armonik.Ffi.Bdn.StrSweep:Run(System.String,int,double,int[],System.String[],System.String[]):int (Tier1-OSR)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1-OSR code
; OSR variant for entry point 0x2db
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; partially interruptible
; with Dynamic PGO: edge weights are invalid, and fgCalledCount is 99.57
; 1 inlinees with PGO data; 36 single block inlinees; 22 inlinees without PGO data

G_M000_IG01:                ;; offset=0x0000
       mov      rax, qword ptr [rbp]
       push     rax
       sub      rsp, 304
       mov      qword ptr [rsp+0x5F8], r15
       mov      qword ptr [rsp+0x5F0], r14
       mov      qword ptr [rsp+0x5E8], r13
       mov      qword ptr [rsp+0x5E0], r12
       mov      qword ptr [rsp+0x5D8], rbx
       vzeroupper 
       lea      rbp, [rsp+0x130]
       mov      r14, gword ptr [rbp+0x4A0]
       mov      ebx, dword ptr [rbp+0x49C]
       mov      r15, gword ptr [rbp+0x488]
       mov      r10, gword ptr [rbp+0x418]
       mov      r8d, dword ptr [rbp+0x414]
       mov      r13, gword ptr [rbp+0x3E8]
       mov      rax, gword ptr [rbp+0x3C8]
       mov      ecx, dword ptr [rbp+0x388]
       mov      r12, qword ptr [rbp+0x358]
 
G_M000_IG02:                ;; offset=0x007C
       mov      gword ptr [rbp+0x418], r10
       test     r10, r10
       je       G_M000_IG83
       mov      dword ptr [rbp+0x414], r8d
       test     r8d, r8d
       jl       SHORT G_M000_IG03
       jmp      G_M000_IG40
 
G_M000_IG03:                ;; offset=0x009D
       mov      r8d, dword ptr [rbp+0x414]
       jmp      G_M000_IG83
 
G_M000_IG04:                ;; offset=0x00A9
       mov      edi, r8d
       mov      rsi, gword ptr [rdx+8*rdi+0x10]
       mov      rdi, rsi
       mov      rsi, r15
       xor      r9d, r9d
       cmp      dword ptr [r15+0x08], 0
       mov      dword ptr [rbp+0x414], r8d
       mov      gword ptr [rbp+0x418], rdx
       mov      gword ptr [rbp+0x408], rdi
       jg       SHORT G_M000_IG06
 
G_M000_IG05:                ;; offset=0x00D6
       mov      r8d, dword ptr [rbp+0x414]
       inc      r8d
       mov      rdx, gword ptr [rbp+0x418]
       mov      edi, dword ptr [rdx+0x08]
       cmp      edi, r8d
       jle      G_M000_IG139
       jmp      SHORT G_M000_IG04
 
G_M000_IG06:                ;; offset=0x00F5
       mov      rdx, rsi
       mov      esi, r9d
 
G_M000_IG07:                ;; offset=0x00FB
       cmp      esi, dword ptr [rdx+0x08]
       jae      G_M000_IG157
       mov      dword ptr [rbp+0x3FC], esi
       mov      edi, esi
       mov      gword ptr [rbp+0x400], rdx
       mov      edi, dword ptr [rdx+4*rdi+0x10]
       mov      dword ptr [rbp+0x3F0], edi
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp+0x3E8], rax
       mov      rdi, gword ptr [rbp+0x408]
       mov      esi, dword ptr [rbp+0x3F0]
       call     [Armonik.Ffi.Bdn.StrSweep:Make(System.String,int):System.String]
       mov      gword ptr [rbp+0x3E0], rax
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0x88], rax
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0x90], rax
       mov      rdi, rax
       call     [Armonik.Ffi.Facade.UploadResultData:.ctor():this]
       mov      rax, gword ptr [rbp-0x90]
       lea      rdi, bword ptr [rax+0x08]
       mov      rsi, gword ptr [rbp+0x3E0]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rax, gword ptr [rbp-0x88]
       lea      rdi, bword ptr [rax+0x08]
       mov      rsi, gword ptr [rbp-0x90]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rax, gword ptr [rbp+0x3E8]
       lea      rdi, bword ptr [rax+0x08]
       mov      rsi, gword ptr [rbp-0x88]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0x98], rax
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0xA0], rax
 
G_M000_IG08:                ;; offset=0x01F3
       mov      rdi, rax
       call     [Armonik.Ffi.Shapes.V1.UploadResultData:.ctor():this]
       mov      rdi, gword ptr [rbp-0xA0]
       mov      rsi, gword ptr [rbp+0x3E0]
       call     [Armonik.Ffi.Shapes.V1.UploadResultData:set_SessionId(System.String):this]
       mov      rax, gword ptr [rbp-0x98]
       lea      rdi, bword ptr [rax+0x10]
       mov      rsi, gword ptr [rbp-0xA0]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, gword ptr [rbp-0x98]
       call     [Google.Protobuf.MessageExtensions:ToByteArray(Google.Protobuf.IMessage):ubyte[]]
       mov      gword ptr [rbp+0x3D8], rax
       mov      rdi, ADDR
       mov      rdi, gword ptr [rdi]
       mov      rsi, gword ptr [rbp+0x3E0]
       call     [System.Text.UTF8Encoding:GetByteCount(System.String):int:this]
       mov      ecx, eax
       mov      dword ptr [rbp+0x3D4], ecx
       xor      ecx, ecx
       mov      rsi, gword ptr [rbp+0x450]
       cmp      dword ptr [rsi+0x08], 0
       jle      G_M000_IG18
 
G_M000_IG09:                ;; offset=0x0270
       lea      rsi, [rbp+0x450]
       mov      dword ptr [rbp+0x39C], ecx
       mov      edi, ecx
       call     [Armonik.Ffi.Bdn.StrSweep:<Run>g__Set|2_0(int,byref)]
       mov      rax, gword ptr [rbp+0x3E8]
       mov      rsi, gword ptr [rax+0x08]
       mov      rdi, gword ptr [rbp+0x468]
       xor      edx, edx
       cmp      dword ptr [rdi], edi
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:EncodeInto(Armonik.Ffi.Facade.UploadResultDataMessage,bool):int:this]
       test     eax, eax
       jl       G_M000_IG50
       mov      rdi, gword ptr [rbp+0x468]
       cmp      dword ptr [rdi], edi
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:ContextBytes():ubyte[]:this]
       test     rax, rax
       jne      SHORT G_M000_IG11
 
G_M000_IG10:                ;; offset=0x02BD
       xor      rdi, rdi
       xor      ecx, ecx
       jmp      SHORT G_M000_IG12
 
G_M000_IG11:                ;; offset=0x02C3
       lea      rdi, bword ptr [rax+0x10]
       mov      ecx, dword ptr [rax+0x08]
 
G_M000_IG12:                ;; offset=0x02CA
       mov      r8, gword ptr [rbp+0x3D8]
       test     r8, r8
       jne      SHORT G_M000_IG14
 
G_M000_IG13:                ;; offset=0x02D6
       xor      rsi, rsi
       xor      r9d, r9d
       jmp      SHORT G_M000_IG15
 
G_M000_IG14:                ;; offset=0x02DD
       lea      rsi, bword ptr [r8+0x10]
       mov      gword ptr [rbp+0x3D8], r8
       mov      r9d, dword ptr [r8+0x08]
       mov      r8, gword ptr [rbp+0x3D8]
 
G_M000_IG15:                ;; offset=0x02F3
       cmp      ecx, r9d
       mov      gword ptr [rbp+0x3D8], r8
       jne      G_M000_IG89
 
G_M000_IG16:                ;; offset=0x0303
       mov      edx, r9d
       call     [System.SpanHelpers:SequenceEqual(byref,byref,ulong):bool]
       test     eax, eax
       je       G_M000_IG89
 
G_M000_IG17:                ;; offset=0x0314
       mov      ecx, dword ptr [rbp+0x39C]
       inc      ecx
       mov      rsi, gword ptr [rbp+0x450]
       cmp      dword ptr [rsi+0x08], ecx
       jg       G_M000_IG09
 
G_M000_IG18:                ;; offset=0x032C
       mov      rsi, gword ptr [rbp+0x450]
       mov      esi, dword ptr [rsi+0x08]
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWARR_1_VC
       mov      gword ptr [rbp+0x3C8], rax
       call     [Armonik.Ffi.Bdn.ProcCpu:Wall():long]
       mov      qword ptr [rbp+0x3C0], rax
       call     [Armonik.Ffi.Bdn.ProcCpu:Wall():long]
       sub      rax, qword ptr [rbp+0x3C0]
       vxorps   xmm0, xmm0, xmm0
       vcvtsi2sd xmm0, xmm0, rax
       vdivsd   xmm0, xmm0, qword ptr [reloc @RWD00]
       vmovsd   xmm1, qword ptr [reloc @RWD08]
       vucomisd xmm1, xmm0
       jbe      G_M000_IG34
 
G_M000_IG19:                ;; offset=0x0389
       xor      eax, eax
       mov      rdx, gword ptr [rbp+0x450]
       cmp      dword ptr [rdx+0x08], 0
       jle      G_M000_IG33
 
G_M000_IG20:                ;; offset=0x039C
       mov      rdx, gword ptr [rbp+0x450]
       cmp      eax, dword ptr [rdx+0x08]
       jae      G_M000_IG157
       mov      edi, eax
       mov      edx, dword ptr [rdx+4*rdi+0x10]
       mov      dword ptr [r12], edx
       mov      rdx, gword ptr [rbp+0x458]
       cmp      eax, dword ptr [rdx+0x08]
       jae      G_M000_IG157
       mov      edi, eax
       mov      edx, dword ptr [rdx+4*rdi+0x10]
       mov      rdi, qword ptr [rbp-0x70]
       mov      dword ptr [rdi], edx
       mov      rdx, gword ptr [rbp+0x460]
       cmp      eax, dword ptr [rdx+0x08]
       jae      G_M000_IG157
       mov      dword ptr [rbp+0x398], eax
       mov      esi, eax
       movzx    rdx, byte  ptr [rdx+rsi+0x10]
       mov      rsi, qword ptr [rbp-0x78]
       mov      byte  ptr [rsi], dl
       mov      qword ptr [rbp-0x30], 1
 
G_M000_IG21:                ;; offset=0x03FD
       call     [Armonik.Ffi.Bdn.ProcCpu:Wall():long]
       mov      qword ptr [rbp-0x38], rax
       xor      ecx, ecx
       mov      qword ptr [rbp-0x40], rcx
       cmp      qword ptr [rbp-0x30], 0
       jle      SHORT G_M000_IG23
 
G_M000_IG22:                ;; offset=0x0414
       mov      r9, gword ptr [rbp+0x3E8]
       mov      rsi, gword ptr [r9+0x08]
       mov      rdi, gword ptr [rbp+0x468]
       xor      edx, edx
       cmp      dword ptr [rdi], edi
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:EncodeInto(Armonik.Ffi.Facade.UploadResultDataMessage,bool):int:this]
       mov      rcx, qword ptr [rbp-0x40]
       inc      rcx
       cmp      rcx, qword ptr [rbp-0x30]
       mov      qword ptr [rbp-0x40], rcx
       jl       SHORT G_M000_IG22
 
G_M000_IG23:                ;; offset=0x0441
       call     [Armonik.Ffi.Bdn.ProcCpu:Wall():long]
       sub      rax, qword ptr [rbp-0x38]
       vxorps   xmm0, xmm0, xmm0
       vcvtsi2sd xmm0, xmm0, rax
       vdivsd   xmm0, xmm0, qword ptr [reloc @RWD00]
       vmovsd   xmm1, qword ptr [rbp+0x470]
       vmulsd   xmm1, xmm1, qword ptr [reloc @RWD16]
       vucomisd xmm0, xmm1
       ja       SHORT G_M000_IG29
 
G_M000_IG24:                ;; offset=0x0472
       mov      rax, qword ptr [rbp-0x30]
       cmp      rax, 0x4000000
       jg       SHORT G_M000_IG26
 
G_M000_IG25:                ;; offset=0x047E
       add      rax, rax
       mov      qword ptr [rbp-0x30], rax
       jmp      G_M000_IG21
 
G_M000_IG26:                ;; offset=0x048A
       mov      qword ptr [rbp-0x30], rax
       jmp      SHORT G_M000_IG29
 
G_M000_IG27:                ;; offset=0x0490
       mov      eax, dword ptr [rbp+0x39C]
       inc      eax
       mov      rdi, gword ptr [rbp+0x450]
       cmp      dword ptr [rdi+0x08], eax
       mov      qword ptr [rbp-0x68], r12
       mov      ecx, eax
       jg       G_M000_IG49
 
G_M000_IG28:                ;; offset=0x04AE
       jmp      G_M000_IG48
 
G_M000_IG29:                ;; offset=0x04B3
       vxorps   xmm1, xmm1, xmm1
       vcvtsi2sd xmm1, xmm1, qword ptr [rbp-0x30]
       vmulsd   xmm1, xmm1, qword ptr [rbp+0x470]
       vmovups  xmm2, xmmword ptr [reloc @RWD32]
       vmaxsd   xmm0, xmm2, xmm0
       vdivsd   xmm0, xmm1, xmm0
       vcvttsd2si  rcx, xmm0
       cmp      rcx, 1
       jle      SHORT G_M000_IG31
 
G_M000_IG30:                ;; offset=0x04E0
       jmp      SHORT G_M000_IG32
 
G_M000_IG31:                ;; offset=0x04E2
       mov      ecx, 1
 
G_M000_IG32:                ;; offset=0x04E7
       mov      eax, dword ptr [rbp+0x398]
       mov      rdx, gword ptr [rbp+0x3C8]
       cmp      eax, dword ptr [rdx+0x08]
       jae      G_M000_IG157
       mov      edi, eax
       mov      gword ptr [rbp+0x3C8], rdx
       mov      qword ptr [rdx+8*rdi+0x10], rcx
       inc      eax
       mov      rcx, gword ptr [rbp+0x450]
       cmp      dword ptr [rcx+0x08], eax
       jg       G_M000_IG20
 
G_M000_IG33:                ;; offset=0x051D
       call     [Armonik.Ffi.Bdn.ProcCpu:Wall():long]
       mov      rcx, qword ptr [rbp+0x3C0]
       sub      rax, rcx
       vxorps   xmm0, xmm0, xmm0
       vcvtsi2sd xmm0, xmm0, rax
       vdivsd   xmm0, xmm0, qword ptr [reloc @RWD00]
       vmovsd   xmm1, qword ptr [reloc @RWD08]
       vucomisd xmm1, xmm0
       mov      qword ptr [rbp+0x3C0], rcx
       ja       G_M000_IG19
 
G_M000_IG34:                ;; offset=0x0557
       mov      r12, gword ptr [rbp+0x450]
       mov      esi, dword ptr [r12+0x08]
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWARR_1_VC
       mov      gword ptr [rbp+0x3B8], rax
       mov      esi, dword ptr [r12+0x08]
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWARR_1_OBJ
       mov      r12, gword ptr [rbp+0x3E8]
       lea      rdi, bword ptr [r12+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       xor      eax, eax
       mov      dword ptr [rbp+0x394], eax
       mov      rdi, gword ptr [rbp+0x450]
       cmp      dword ptr [rdi+0x08], 0
       jle      SHORT G_M000_IG36
 
G_M000_IG35:                ;; offset=0x05B6
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      rcx, gword ptr [r12+0x10]
       mov      gword ptr [rbp-0xC0], rcx
       mov      gword ptr [rbp-0xB8], rax
       mov      rdi, rax
       call     [System.Collections.Generic.List`1[double]:.ctor():this]
       movsxd   rsi, dword ptr [rbp+0x394]
       mov      rdi, gword ptr [rbp-0xC0]
       mov      rdx, gword ptr [rbp-0xB8]
       call     CORINFO_HELP_ARRADDR_ST
       mov      eax, dword ptr [rbp+0x394]
       inc      eax
       mov      rcx, gword ptr [rbp+0x450]
       cmp      dword ptr [rcx+0x08], eax
       mov      dword ptr [rbp+0x394], eax
       jg       SHORT G_M000_IG35
 
G_M000_IG36:                ;; offset=0x0615
       xor      r8d, r8d
       xor      ecx, ecx
       test     ebx, ebx
       mov      gword ptr [rbp+0x438], r13
       mov      r8d, ecx
       mov      r13, r12
       jle      G_M000_IG98
 
G_M000_IG37:                ;; offset=0x062F
       xor      ecx, ecx
       xor      eax, eax
       mov      rdi, gword ptr [rbp+0x450]
       cmp      dword ptr [rdi+0x08], 0
       mov      dword ptr [rbp+0x390], r8d
       mov      ecx, eax
       jg       SHORT G_M000_IG39
 
G_M000_IG38:                ;; offset=0x0649
       mov      r8d, dword ptr [rbp+0x390]
       inc      r8d
       cmp      r8d, ebx
       jge      G_M000_IG98
       jmp      SHORT G_M000_IG37
 
G_M000_IG39:                ;; offset=0x065E
       mov      dword ptr [rbp+0x38C], ecx
       mov      r8d, dword ptr [rbp+0x390]
       lea      eax, [rcx+r8]
       mov      rsi, gword ptr [rbp+0x450]
       cdq      
       idiv     edx:eax, dword ptr [rsi+0x08]
       mov      eax, edx
       lea      rsi, [rbp+0x450]
       mov      dword ptr [rbp+0x388], eax
       mov      edi, eax
       call     [Armonik.Ffi.Bdn.StrSweep:<Run>g__Set|2_0(int,byref)]
       call     [Armonik.Ffi.Bdn.ProcCpu:Ns():long]
       mov      qword ptr [rbp+0x380], rax
       call     [Armonik.Ffi.Bdn.ProcCpu:Wall():long]
       xor      ecx, ecx
       mov      qword ptr [rbp+0x378], rax
       mov      r12, rcx
       mov      rax, gword ptr [rbp+0x3C8]
       mov      ecx, dword ptr [rbp+0x388]
 
G_M000_IG40:                ;; offset=0x06BD
       mov      esi, dword ptr [rax+0x08]
       cmp      ecx, esi
       jae      G_M000_IG157
       mov      dword ptr [rbp+0x388], ecx
       mov      esi, ecx
       mov      qword ptr [rbp-0x80], rsi
       mov      gword ptr [rbp+0x3C8], rax
       cmp      qword ptr [rax+8*rsi+0x10], r12
       jle      G_M000_IG91
 
G_M000_IG41:                ;; offset=0x06E6
       mov      rsi, gword ptr [r13+0x08]
       mov      rdi, gword ptr [rbp+0x468]
       xor      edx, edx
       cmp      dword ptr [rdi], edi
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:EncodeInto(Armonik.Ffi.Facade.UploadResultDataMessage,bool):int:this]
       inc      r12
       mov      rax, gword ptr [rbp+0x3C8]
       mov      rcx, qword ptr [rbp-0x80]
       cmp      qword ptr [rax+8*rcx+0x10], r12
       jle      SHORT G_M000_IG42
       mov      gword ptr [rbp+0x3C8], rax
       jmp      SHORT G_M000_IG41
 
G_M000_IG42:                ;; offset=0x0719
       mov      gword ptr [rbp+0x3C8], rax
       jmp      G_M000_IG91
 
G_M000_IG43:                ;; offset=0x0725
       cmp      r8d, esi
       jae      G_M000_IG157
       mov      edi, r8d
       mov      rsi, gword ptr [rdx+8*rdi+0x10]
       mov      rdi, r15
       xor      r9d, r9d
       cmp      dword ptr [r15+0x08], 0
       mov      dword ptr [rbp+0x414], r8d
       mov      gword ptr [rbp+0x418], rdx
       mov      qword ptr [rbp-0x68], r12
       mov      gword ptr [rbp+0x408], rsi
       jg       SHORT G_M000_IG45
 
G_M000_IG44:                ;; offset=0x075C
       mov      r8d, dword ptr [rbp+0x414]
       inc      r8d
       mov      rdi, gword ptr [rbp+0x418]
       mov      esi, dword ptr [rdi+0x08]
       cmp      esi, r8d
       jle      G_M000_IG139
       mov      rdx, rdi
       mov      r12, qword ptr [rbp-0x68]
       jmp      SHORT G_M000_IG43
 
G_M000_IG45:                ;; offset=0x0782
       mov      esi, r9d
       mov      r8, rdi
 
G_M000_IG46:                ;; offset=0x0788
       cmp      esi, dword ptr [r8+0x08]
       jae      G_M000_IG157
       mov      dword ptr [rbp+0x3FC], esi
       mov      edi, esi
       mov      gword ptr [rbp+0x400], r8
       mov      edi, dword ptr [r8+4*rdi+0x10]
       mov      dword ptr [rbp+0x3F0], edi
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp+0x3E8], rax
       mov      rdi, gword ptr [rbp+0x408]
       mov      esi, dword ptr [rbp+0x3F0]
       call     [Armonik.Ffi.Bdn.StrSweep:Make(System.String,int):System.String]
       mov      gword ptr [rbp+0x3E0], rax
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0x88], rax
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0x90], rax
       mov      rdi, rax
       call     [Armonik.Ffi.Facade.UploadResultData:.ctor():this]
       mov      rax, gword ptr [rbp-0x90]
       lea      rdi, bword ptr [rax+0x08]
       mov      rsi, gword ptr [rbp+0x3E0]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rax, gword ptr [rbp-0x88]
       lea      rdi, bword ptr [rax+0x08]
       mov      rsi, gword ptr [rbp-0x90]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rax, gword ptr [rbp+0x3E8]
       lea      rdi, bword ptr [rax+0x08]
       mov      rsi, gword ptr [rbp-0x88]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0x98], rax
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0xA0], rax
 
G_M000_IG47:                ;; offset=0x0882
       mov      rdi, rax
       call     [Armonik.Ffi.Shapes.V1.UploadResultData:.ctor():this]
       mov      rdi, gword ptr [rbp-0xA0]
       mov      rsi, gword ptr [rbp+0x3E0]
       call     [Armonik.Ffi.Shapes.V1.UploadResultData:set_SessionId(System.String):this]
       mov      rax, gword ptr [rbp-0x98]
       lea      rdi, bword ptr [rax+0x10]
       mov      rsi, gword ptr [rbp-0xA0]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, gword ptr [rbp-0x98]
       call     [Google.Protobuf.MessageExtensions:ToByteArray(Google.Protobuf.IMessage):ubyte[]]
       mov      gword ptr [rbp+0x3D8], rax
       mov      rdi, ADDR
       mov      rdi, gword ptr [rdi]
       mov      rsi, gword ptr [rbp+0x3E0]
       call     [System.Text.UTF8Encoding:GetByteCount(System.String):int:this]
       mov      ecx, eax
       mov      dword ptr [rbp+0x3D4], ecx
       xor      esi, esi
       xor      ecx, ecx
       mov      rsi, gword ptr [rbp+0x450]
       cmp      dword ptr [rsi+0x08], 0
       jg       SHORT G_M000_IG49
 
G_M000_IG48:                ;; offset=0x08FD
       mov      rsi, gword ptr [rbp+0x450]
       mov      esi, dword ptr [rsi+0x08]
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWARR_1_VC
       mov      gword ptr [rbp+0x3C8], rax
       call     [Armonik.Ffi.Bdn.ProcCpu:Wall():long]
       mov      qword ptr [rbp+0x3C0], rax
       call     [Armonik.Ffi.Bdn.ProcCpu:Wall():long]
       sub      rax, qword ptr [rbp+0x3C0]
       vxorps   xmm0, xmm0, xmm0
       vcvtsi2sd xmm0, xmm0, rax
       vdivsd   xmm0, xmm0, qword ptr [reloc @RWD00]
       vmovsd   xmm1, qword ptr [reloc @RWD08]
       vucomisd xmm1, xmm0
       jbe      G_M000_IG71
       jmp      G_M000_IG61
 
G_M000_IG49:                ;; offset=0x095F
       lea      rsi, [rbp+0x450]
       mov      dword ptr [rbp+0x39C], ecx
       mov      edi, ecx
       call     [Armonik.Ffi.Bdn.StrSweep:<Run>g__Set|2_0(int,byref)]
       mov      rax, gword ptr [rbp+0x3E8]
       mov      rsi, gword ptr [rax+0x08]
       mov      rdi, gword ptr [rbp+0x468]
       xor      edx, edx
       cmp      dword ptr [rdi], edi
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:EncodeInto(Armonik.Ffi.Facade.UploadResultDataMessage,bool):int:this]
       test     eax, eax
       jge      SHORT G_M000_IG52
 
G_M000_IG50:                ;; offset=0x0994
       call     [System.Console:get_Error():System.IO.TextWriter]
       mov      r15, rax
       mov      r13d, dword ptr [rbp+0x39C]
       mov      rbx, gword ptr [rbp+0x448]
       cmp      r13d, dword ptr [rbx+0x08]
       jae      G_M000_IG157
       mov      esi, r13d
       mov      rsi, gword ptr [rbx+8*rsi+0x10]
       mov      rdi, ADDR
       call     [System.String:Concat(System.String,System.String):System.String]
       mov      rsi, rax
       mov      rdi, r15
       mov      rax, qword ptr [r15]
       mov      rax, qword ptr [rax+0x68]
       call     [rax+0x30]System.IO.TextWriter:WriteLine(System.String):this
       mov      eax, 1
 
G_M000_IG51:                ;; offset=0x09E2
       add      rsp, 0x5D8
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG52:                ;; offset=0x09F4
       mov      r12, qword ptr [rbp-0x68]
       mov      rdi, gword ptr [rbp+0x468]
       cmp      dword ptr [rdi], edi
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:ContextBytes():ubyte[]:this]
       test     rax, rax
       jne      SHORT G_M000_IG54
 
G_M000_IG53:                ;; offset=0x0A0C
       xor      rdi, rdi
       xor      ecx, ecx
       jmp      SHORT G_M000_IG55
 
G_M000_IG54:                ;; offset=0x0A12
       lea      rdi, bword ptr [rax+0x10]
       mov      ecx, dword ptr [rax+0x08]
 
G_M000_IG55:                ;; offset=0x0A19
       mov      rax, gword ptr [rbp+0x3D8]
       test     rax, rax
       jne      SHORT G_M000_IG57
 
G_M000_IG56:                ;; offset=0x0A25
       xor      rsi, rsi
       xor      r9d, r9d
       jmp      SHORT G_M000_IG58
 
G_M000_IG57:                ;; offset=0x0A2C
       lea      rsi, bword ptr [rax+0x10]
       mov      gword ptr [rbp+0x3D8], rax
       mov      r9d, dword ptr [rax+0x08]
       mov      rax, gword ptr [rbp+0x3D8]
 
G_M000_IG58:                ;; offset=0x0A42
       cmp      ecx, r9d
       mov      gword ptr [rbp+0x3D8], rax
       jne      G_M000_IG114
 
G_M000_IG59:                ;; offset=0x0A52
       mov      edx, r9d
       call     [System.SpanHelpers:SequenceEqual(byref,byref,ulong):bool]
       test     eax, eax
       je       G_M000_IG114
 
G_M000_IG60:                ;; offset=0x0A63
       jmp      G_M000_IG27
 
G_M000_IG61:                ;; offset=0x0A68
       xor      eax, eax
       mov      rdx, gword ptr [rbp+0x450]
       cmp      dword ptr [rdx+0x08], 0
       jg       SHORT G_M000_IG64
 
G_M000_IG62:                ;; offset=0x0A77
       call     [Armonik.Ffi.Bdn.ProcCpu:Wall():long]
       mov      rdi, qword ptr [rbp+0x3C0]
       sub      rax, rdi
       vxorps   xmm0, xmm0, xmm0
       vcvtsi2sd xmm0, xmm0, rax
       vdivsd   xmm0, xmm0, qword ptr [reloc @RWD00]
       vmovsd   xmm1, qword ptr [reloc @RWD08]
       vucomisd xmm1, xmm0
       mov      qword ptr [rbp+0x3C0], rdi
       ja       SHORT G_M000_IG61
 
G_M000_IG63:                ;; offset=0x0AAD
       jmp      G_M000_IG71
 
G_M000_IG64:                ;; offset=0x0AB2
       mov      rdx, gword ptr [rbp+0x450]
       cmp      eax, dword ptr [rdx+0x08]
       jae      G_M000_IG157
       mov      edi, eax
       mov      edx, dword ptr [rdx+4*rdi+0x10]
       mov      rdi, qword ptr [rbp-0x68]
       mov      dword ptr [rdi], edx
       mov      rdx, gword ptr [rbp+0x458]
       cmp      eax, dword ptr [rdx+0x08]
       jae      G_M000_IG157
       mov      esi, eax
       mov      edx, dword ptr [rdx+4*rsi+0x10]
       mov      rsi, qword ptr [rbp-0x70]
       mov      dword ptr [rsi], edx
       mov      rdx, gword ptr [rbp+0x460]
       cmp      eax, dword ptr [rdx+0x08]
       jae      G_M000_IG157
       mov      dword ptr [rbp+0x398], eax
       mov      r8d, eax
       movzx    rdx, byte  ptr [rdx+r8+0x10]
       mov      r8, qword ptr [rbp-0x78]
       mov      byte  ptr [r8], dl
       mov      qword ptr [rbp-0x30], 1
 
G_M000_IG65:                ;; offset=0x0B18
       call     [Armonik.Ffi.Bdn.ProcCpu:Wall():long]
       mov      qword ptr [rbp-0x38], rax
       xor      ecx, ecx
       mov      qword ptr [rbp-0x40], rcx
       cmp      qword ptr [rbp-0x30], 0
       jle      SHORT G_M000_IG67
 
G_M000_IG66:                ;; offset=0x0B2F
       mov      r9, gword ptr [rbp+0x3E8]
       mov      rsi, gword ptr [r9+0x08]
       mov      rdi, gword ptr [rbp+0x468]
       xor      edx, edx
       cmp      dword ptr [rdi], edi
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:EncodeInto(Armonik.Ffi.Facade.UploadResultDataMessage,bool):int:this]
       mov      rcx, qword ptr [rbp-0x40]
       inc      rcx
       cmp      rcx, qword ptr [rbp-0x30]
       mov      qword ptr [rbp-0x40], rcx
       jl       SHORT G_M000_IG66
 
G_M000_IG67:                ;; offset=0x0B5C
       call     [Armonik.Ffi.Bdn.ProcCpu:Wall():long]
       sub      rax, qword ptr [rbp-0x38]
       vxorps   xmm0, xmm0, xmm0
       vcvtsi2sd xmm0, xmm0, rax
       vdivsd   xmm0, xmm0, qword ptr [reloc @RWD00]
       vmovsd   xmm1, qword ptr [rbp+0x470]
       vmulsd   xmm1, xmm1, qword ptr [reloc @RWD16]
       vucomisd xmm0, xmm1
       ja       G_M000_IG72
 
G_M000_IG68:                ;; offset=0x0B91
       mov      rax, qword ptr [rbp-0x30]
       cmp      rax, 0x4000000
       jg       SHORT G_M000_IG70
 
G_M000_IG69:                ;; offset=0x0B9D
       add      rax, rax
       mov      qword ptr [rbp-0x30], rax
       jmp      G_M000_IG65
 
G_M000_IG70:                ;; offset=0x0BA9
       mov      qword ptr [rbp-0x30], rax
       jmp      SHORT G_M000_IG72
 
G_M000_IG71:                ;; offset=0x0BAF
       mov      rax, gword ptr [rbp+0x450]
       mov      gword ptr [rbp-0x128], rax
       mov      esi, dword ptr [rax+0x08]
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWARR_1_VC
       mov      gword ptr [rbp+0x3B8], rax
       mov      rsi, gword ptr [rbp-0x128]
       mov      esi, dword ptr [rsi+0x08]
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWARR_1_OBJ
       mov      rcx, gword ptr [rbp+0x3E8]
       lea      rdi, bword ptr [rcx+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       xor      eax, eax
       mov      dword ptr [rbp+0x394], eax
       mov      rcx, gword ptr [rbp+0x450]
       cmp      dword ptr [rcx+0x08], 0
       jle      G_M000_IG78
       jmp      SHORT G_M000_IG77
 
G_M000_IG72:                ;; offset=0x0C1D
       vxorps   xmm1, xmm1, xmm1
       vcvtsi2sd xmm1, xmm1, qword ptr [rbp-0x30]
       vmulsd   xmm1, xmm1, qword ptr [rbp+0x470]
       vmovups  xmm2, xmmword ptr [reloc @RWD32]
       vmaxsd   xmm0, xmm2, xmm0
       vdivsd   xmm0, xmm1, xmm0
       vcvttsd2si  rcx, xmm0
       cmp      rcx, 1
       jle      SHORT G_M000_IG74
 
G_M000_IG73:                ;; offset=0x0C4A
       jmp      SHORT G_M000_IG75
 
G_M000_IG74:                ;; offset=0x0C4C
       mov      ecx, 1
 
G_M000_IG75:                ;; offset=0x0C51
       mov      eax, dword ptr [rbp+0x398]
       mov      rdx, gword ptr [rbp+0x3C8]
       cmp      eax, dword ptr [rdx+0x08]
       jae      G_M000_IG157
       mov      edi, eax
       mov      gword ptr [rbp+0x3C8], rdx
       mov      qword ptr [rdx+8*rdi+0x10], rcx
       inc      eax
       mov      rcx, gword ptr [rbp+0x450]
       cmp      dword ptr [rcx+0x08], eax
       jg       G_M000_IG64
 
G_M000_IG76:                ;; offset=0x0C87
       jmp      G_M000_IG62
 
G_M000_IG77:                ;; offset=0x0C8C
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      rcx, gword ptr [rbp+0x3E8]
       mov      rdi, gword ptr [rcx+0x10]
       mov      gword ptr [rbp-0xC0], rdi
       mov      gword ptr [rbp-0xB8], rax
       mov      rdi, rax
       call     [System.Collections.Generic.List`1[double]:.ctor():this]
       movsxd   rsi, dword ptr [rbp+0x394]
       mov      rdi, gword ptr [rbp-0xC0]
       mov      rdx, gword ptr [rbp-0xB8]
       call     CORINFO_HELP_ARRADDR_ST
       mov      eax, dword ptr [rbp+0x394]
       inc      eax
       mov      rcx, gword ptr [rbp+0x450]
       cmp      dword ptr [rcx+0x08], eax
       mov      dword ptr [rbp+0x394], eax
       jg       SHORT G_M000_IG77
 
G_M000_IG78:                ;; offset=0x0CF1
       xor      eax, eax
       test     ebx, ebx
       mov      gword ptr [rbp+0x438], r13
       mov      r12, gword ptr [rbp+0x440]
       mov      r13, gword ptr [rbp+0x3E8]
       jg       SHORT G_M000_IG80
 
G_M000_IG79:                ;; offset=0x0D0C
       mov      rcx, gword ptr [rbp+0x3C8]
       mov      edi, dword ptr [rcx+0x08]
       dec      edi
       cmp      edi, 1
       jge      G_M000_IG124
       jmp      G_M000_IG123
 
G_M000_IG80:                ;; offset=0x0D26
       xor      edi, edi
       mov      rsi, gword ptr [rbp+0x450]
       cmp      dword ptr [rsi+0x08], 0
       mov      dword ptr [rbp+0x390], eax
       mov      ecx, edi
       jg       SHORT G_M000_IG82
 
G_M000_IG81:                ;; offset=0x0D3D
       mov      eax, dword ptr [rbp+0x390]
       inc      eax
       cmp      eax, ebx
       jge      SHORT G_M000_IG79
       jmp      SHORT G_M000_IG80
 
G_M000_IG82:                ;; offset=0x0D4B
       mov      dword ptr [rbp+0x38C], ecx
       mov      r8d, dword ptr [rbp+0x390]
       lea      eax, [rcx+r8]
       mov      rsi, gword ptr [rbp+0x450]
       cdq      
       idiv     edx:eax, dword ptr [rsi+0x08]
       mov      eax, edx
       lea      rsi, [rbp+0x450]
       mov      dword ptr [rbp+0x388], eax
       mov      edi, eax
       call     [Armonik.Ffi.Bdn.StrSweep:<Run>g__Set|2_0(int,byref)]
       call     [Armonik.Ffi.Bdn.ProcCpu:Ns():long]
       mov      qword ptr [rbp+0x380], rax
       call     [Armonik.Ffi.Bdn.ProcCpu:Wall():long]
       mov      rdi, rax
       xor      ecx, ecx
       xor      esi, esi
       mov      qword ptr [rbp+0x378], rdi
       mov      gword ptr [rbp+0x440], r12
       mov      r12, rsi
       mov      rax, gword ptr [rbp+0x3C8]
       mov      ecx, dword ptr [rbp+0x388]
       mov      r8d, dword ptr [rbp+0x414]
 
G_M000_IG83:                ;; offset=0x0DBD
       cmp      ecx, dword ptr [rax+0x08]
       jae      G_M000_IG157
       mov      dword ptr [rbp+0x388], ecx
       mov      esi, ecx
       mov      gword ptr [rbp+0x3C8], rax
       cmp      qword ptr [rax+8*rsi+0x10], r12
       mov      dword ptr [rbp+0x414], r8d
       jle      G_M000_IG116
 
G_M000_IG84:                ;; offset=0x0DE7
       mov      rsi, gword ptr [r13+0x08]
       mov      rdi, gword ptr [rbp+0x468]
       xor      edx, edx
       cmp      dword ptr [rdi], edi
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:EncodeInto(Armonik.Ffi.Facade.UploadResultDataMessage,bool):int:this]
       inc      r12
       mov      eax, dword ptr [rbp+0x388]
       mov      rcx, gword ptr [rbp+0x3C8]
       cmp      eax, dword ptr [rcx+0x08]
       jae      G_M000_IG157
       mov      edx, eax
       mov      gword ptr [rbp+0x3C8], rcx
       cmp      qword ptr [rcx+8*rdx+0x10], r12
       jle      SHORT G_M000_IG85
       mov      dword ptr [rbp+0x388], eax
       jmp      SHORT G_M000_IG84
 
G_M000_IG85:                ;; offset=0x0E2D
       mov      dword ptr [rbp+0x388], eax
       jmp      G_M000_IG116
 
G_M000_IG86:                ;; offset=0x0E38
       mov      rdi, gword ptr [rbp+0x310]
       call     [System.Console:WriteLine(System.String)]
 
G_M000_IG87:                ;; offset=0x0E45
       lea      rdi, [rbp+0x308]
       mov      rsi, ADDR
       call     [System.Collections.Generic.List`1+Enumerator[System.__Canon]:MoveNext():bool:this]
       test     eax, eax
       jne      SHORT G_M000_IG86
       xor      eax, eax
       test     r15d, r15d
       setne    al
 
G_M000_IG88:                ;; offset=0x0E68
       add      rsp, 0x5D8
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG89:                ;; offset=0x0E7A
       call     [System.Console:get_Error():System.IO.TextWriter]
       mov      gword ptr [rbp-0xB0], rax
       mov      rdi, ADDR
       mov      esi, 6
       call     CORINFO_HELP_NEWARR_1_OBJ
       mov      rdi, ADDR
       mov      gword ptr [rax+0x10], rdi
       mov      gword ptr [rbp-0xA8], rax
       lea      rdi, bword ptr [rax+0x18]
       mov      rsi, gword ptr [rbp+0x408]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      rax, gword ptr [rbp-0xA8]
       mov      gword ptr [rax+0x20], rdi
       mov      edi, dword ptr [rbp+0x3F0]
       call     [System.Number:Int32ToDecStr(int):System.String]
       mov      rcx, gword ptr [rbp-0xA8]
       lea      rdi, bword ptr [rcx+0x28]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      rax, gword ptr [rbp-0xA8]
       mov      gword ptr [rax+0x30], rdi
       mov      ecx, dword ptr [rbp+0x39C]
       mov      rdx, gword ptr [rbp+0x448]
       cmp      ecx, dword ptr [rdx+0x08]
       jae      G_M000_IG157
       mov      dword ptr [rbp+0x39C], ecx
       mov      edi, ecx
       mov      gword ptr [rbp+0x448], rdx
       mov      rsi, gword ptr [rdx+8*rdi+0x10]
       mov      gword ptr [rbp-0xA8], rax
       lea      rdi, bword ptr [rax+0x38]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, gword ptr [rbp-0xA8]
       call     [System.String:Concat(System.String[]):System.String]
       mov      rsi, rax
       mov      rdi, gword ptr [rbp-0xB0]
       mov      rax, qword ptr [rdi]
       mov      rax, qword ptr [rax+0x68]
       call     [rax+0x30]System.IO.TextWriter:WriteLine(System.String):this
       mov      r11d, dword ptr [rbp+0x430]
       inc      r11d
       mov      dword ptr [rbp+0x430], r11d
 
G_M000_IG90:                ;; offset=0x0F75
       jmp      G_M000_IG17
 
G_M000_IG91:                ;; offset=0x0F7A
       call     [Armonik.Ffi.Bdn.ProcCpu:Ns():long]
       mov      r12, rax
       call     [Armonik.Ffi.Bdn.ProcCpu:Wall():long]
       sub      r12, qword ptr [rbp+0x380]
       vxorps   xmm0, xmm0, xmm0
       vcvtsi2sd xmm0, xmm0, r12
       mov      r12, gword ptr [rbp+0x3C8]
       mov      rcx, qword ptr [rbp-0x80]
       vxorps   xmm1, xmm1, xmm1
       vcvtsi2sd xmm1, xmm1, qword ptr [r12+8*rcx+0x10]
       vdivsd   xmm0, xmm0, xmm1
       sub      rax, qword ptr [rbp+0x378]
       vxorps   xmm1, xmm1, xmm1
       vcvtsi2sd xmm1, xmm1, rax
       vxorps   xmm2, xmm2, xmm2
       vcvtsi2sd xmm2, xmm2, qword ptr [r12+8*rcx+0x10]
       vdivsd   xmm1, xmm1, xmm2
       vmovsd   qword ptr [rbp+0x360], xmm1
       mov      rax, gword ptr [r13+0x10]
       mov      edx, dword ptr [rbp+0x388]
       cmp      edx, dword ptr [rax+0x08]
       jae      G_M000_IG157
       mov      rdi, gword ptr [rax+8*rcx+0x10]
       inc      dword ptr [rdi+0x14]
       mov      rsi, gword ptr [rdi+0x08]
       mov      r8d, dword ptr [rdi+0x10]
       cmp      dword ptr [rsi+0x08], r8d
       jbe      SHORT G_M000_IG92
       lea      eax, [r8+0x01]
       mov      dword ptr [rdi+0x10], eax
       mov      edi, r8d
       vmovsd   qword ptr [rsi+8*rdi+0x10], xmm0
       vmovsd   qword ptr [rbp+0x368], xmm0
       jmp      SHORT G_M000_IG93
 
G_M000_IG92:                ;; offset=0x101D
       vmovsd   qword ptr [rbp+0x368], xmm0
       call     [System.Collections.Generic.List`1[double]:AddWithResize(double):this]
 
G_M000_IG93:                ;; offset=0x102B
       mov      rdi, ADDR
       mov      esi, 8
       call     CORINFO_HELP_NEWARR_1_OBJ
       mov      gword ptr [rbp-0x118], rax
       lea      rdi, bword ptr [rax+0x10]
       mov      rsi, gword ptr [rbp+0x408]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      ecx, dword ptr [rbp+0x3F0]
       mov      dword ptr [rax+0x08], ecx
       mov      rdx, gword ptr [rbp-0x118]
       lea      rdi, bword ptr [rdx+0x18]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      ecx, dword ptr [rbp+0x3D4]
       mov      dword ptr [rax+0x08], ecx
       mov      rdx, gword ptr [rbp-0x118]
       lea      rdi, bword ptr [rdx+0x20]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      eax, dword ptr [rbp+0x388]
       mov      rcx, gword ptr [rbp+0x448]
       cmp      eax, dword ptr [rcx+0x08]
       jae      G_M000_IG157
       mov      gword ptr [rbp+0x448], rcx
       mov      rax, qword ptr [rbp-0x80]
       mov      rsi, gword ptr [rcx+8*rax+0x10]
       mov      rdx, gword ptr [rbp-0x118]
       lea      rdi, bword ptr [rdx+0x28]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      ecx, dword ptr [rbp+0x390]
       lea      edi, [rcx+0x01]
       mov      dword ptr [rax+0x08], edi
       mov      rdx, gword ptr [rbp-0x118]
       lea      rdi, bword ptr [rdx+0x30]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
 
G_M000_IG94:                ;; offset=0x1110
       mov      rdi, ADDR
       mov      rdi, gword ptr [rdi]
       vmovsd   xmm0, qword ptr [rbp+0x368]
       vmovsd   qword ptr [rbp-0x48], xmm0
       call     [System.Globalization.NumberFormatInfo:<GetInstance>g__GetProviderNonNull|58_0(System.IFormatProvider):System.Globalization.NumberFormatInfo]
       vmovsd   xmm0, qword ptr [rbp-0x48]
       mov      rsi, rax
       mov      rdi, ADDR
       call     [System.Number:FormatDouble(double,System.String,System.Globalization.NumberFormatInfo):System.String]
       mov      rcx, gword ptr [rbp-0x118]
       lea      rdi, bword ptr [rcx+0x38]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      rdi, gword ptr [rdi]
       vmovsd   xmm1, qword ptr [rbp+0x360]
       vmovsd   qword ptr [rbp-0x50], xmm1
       call     [System.Globalization.NumberFormatInfo:<GetInstance>g__GetProviderNonNull|58_0(System.IFormatProvider):System.Globalization.NumberFormatInfo]
       vmovsd   xmm0, qword ptr [rbp-0x50]
       mov      rsi, rax
       mov      rdi, ADDR
       call     [System.Number:FormatDouble(double,System.String,System.Globalization.NumberFormatInfo):System.String]
       mov      rcx, gword ptr [rbp-0x118]
       lea      rdi, bword ptr [rcx+0x40]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp+0x3C8], r12
       mov      rdi, qword ptr [rbp-0x80]
       mov      rdi, qword ptr [r12+8*rdi+0x10]
       mov      qword ptr [rax+0x08], rdi
       mov      rcx, gword ptr [rbp-0x118]
       lea      rdi, bword ptr [rcx+0x48]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      esi, 1
       mov      rdx, gword ptr [rbp-0x118]
       call     [System.String:JoinCore(System.ReadOnlySpan`1[ushort],System.Object[]):System.String]
       mov      rcx, gword ptr [rbp+0x440]
       inc      dword ptr [rcx+0x14]
       mov      rdx, gword ptr [rcx+0x08]
       mov      r8d, dword ptr [rcx+0x10]
 
G_M000_IG95:                ;; offset=0x120A
       cmp      dword ptr [rdx+0x08], r8d
       jbe      SHORT G_M000_IG96
       lea      edi, [r8+0x01]
       mov      gword ptr [rbp+0x440], rcx
       mov      dword ptr [rcx+0x10], edi
       cmp      r8d, dword ptr [rdx+0x08]
       jae      G_M000_IG157
       mov      edi, r8d
       lea      rdi, bword ptr [rdx+8*rdi+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       jmp      SHORT G_M000_IG97
 
G_M000_IG96:                ;; offset=0x123A
       mov      gword ptr [rbp+0x440], rcx
       mov      rdi, rcx
       mov      rsi, rax
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG97:                ;; offset=0x124D
       mov      ecx, dword ptr [rbp+0x38C]
       inc      ecx
       mov      rax, gword ptr [rbp+0x450]
       cmp      dword ptr [rax+0x08], ecx
       jg       G_M000_IG39
       jmp      G_M000_IG38
 
G_M000_IG98:                ;; offset=0x126A
       mov      r12, gword ptr [rbp+0x3C8]
       mov      ecx, dword ptr [r12+0x08]
       dec      ecx
       cmp      ecx, 1
       jge      SHORT G_M000_IG99
       jmp      SHORT G_M000_IG100
 
G_M000_IG99:                ;; offset=0x127F
       mov      ecx, 1
 
G_M000_IG100:                ;; offset=0x1284
       cmp      ecx, dword ptr [r12+0x08]
       jae      G_M000_IG157
       mov      edi, ecx
       mov      r12, qword ptr [r12+8*rdi+0x10]
       cmp      r12, 0x3E8
       jle      SHORT G_M000_IG101
       jmp      SHORT G_M000_IG102
 
G_M000_IG101:                ;; offset=0x12A1
       mov      r12d, 0x3E8
 
G_M000_IG102:                ;; offset=0x12A7
       xor      eax, eax
       mov      dword ptr [rbp+0x354], eax
 
G_M000_IG103:                ;; offset=0x12AF
       xor      edi, edi
       mov      qword ptr [rbp-0x58], rdi
       lea      rdi, [rbp-0x58]
       mov      rsi, gword ptr [rbp+0x3E0]
       mov      edx, 3
       call     [System.Runtime.InteropServices.GCHandle:.ctor(System.Object,int):this]
       mov      rdi, qword ptr [rbp-0x58]
       mov      qword ptr [rbp+0x348], rdi
       lea      rdi, [rbp+0x348]
       call     [System.Runtime.InteropServices.GCHandle:Free():this]
       mov      eax, dword ptr [rbp+0x354]
       inc      eax
       cmp      eax, 3
       mov      dword ptr [rbp+0x354], eax
       jl       SHORT G_M000_IG103
       call     [Armonik.Ffi.Bdn.ProcCpu:Ns():long]
       mov      qword ptr [rbp+0x3A8], rax
       xor      ecx, ecx
       jmp      SHORT G_M000_IG105
 
G_M000_IG104:                ;; offset=0x1307
       xor      edi, edi
       mov      qword ptr [rbp-0x60], rdi
       lea      rdi, [rbp-0x60]
       mov      rsi, gword ptr [rbp+0x3E0]
       mov      edx, 3
       call     [System.Runtime.InteropServices.GCHandle:.ctor(System.Object,int):this]
       mov      rdi, qword ptr [rbp-0x60]
       mov      qword ptr [rbp+0x338], rdi
       lea      rdi, [rbp+0x338]
       call     [System.Runtime.InteropServices.GCHandle:Free():this]
       mov      rcx, qword ptr [rbp+0x340]
       inc      rcx
 
G_M000_IG105:                ;; offset=0x1345
       mov      qword ptr [rbp+0x340], rcx
       cmp      rcx, r12
       jl       SHORT G_M000_IG104
       call     [Armonik.Ffi.Bdn.ProcCpu:Ns():long]
       sub      rax, qword ptr [rbp+0x3A8]
       vxorps   xmm0, xmm0, xmm0
       vcvtsi2sd xmm0, xmm0, rax
       vxorps   xmm1, xmm1, xmm1
       vcvtsi2sd xmm1, xmm1, r12
       vdivsd   xmm0, xmm0, xmm1
       vmovsd   qword ptr [rbp+0x3A0], xmm0
       xor      r12d, r12d
       jmp      G_M000_IG108
 
G_M000_IG106:                ;; offset=0x1384
       mov      rax, gword ptr [r13+0x10]
       cmp      r12d, dword ptr [rax+0x08]
       jae      G_M000_IG157
       mov      edi, r12d
       mov      rcx, gword ptr [rax+8*rdi+0x10]
       mov      gword ptr [rbp-0xC8], rcx
       mov      rdi, ADDR
       mov      esi, 79
       call     CORINFO_HELP_GETSHARED_NONGCSTATIC_BASE
       mov      rdi, ADDR
       mov      rax, gword ptr [rdi]
       mov      rcx, gword ptr [rbp-0xC8]
       mov      gword ptr [rbp-0xD0], rcx
       mov      rsi, rax
       test     rsi, rsi
       mov      gword ptr [rbp-0xD8], rsi
       jne      G_M000_IG107
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0xE0], rax
       mov      rdi, ADDR
       mov      esi, 79
       call     CORINFO_HELP_GETSHARED_NONGCSTATIC_BASE
       mov      rdi, ADDR
       mov      rsi, gword ptr [rdi]
       test     rsi, rsi
       je       G_M000_IG141
       mov      rax, gword ptr [rbp-0xE0]
       lea      rdi, bword ptr [rax+0x08]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      rsi, gword ptr [rbp-0xE0]
       mov      qword ptr [rsi+0x18], rdi
       mov      rdi, ADDR
       mov      esi, 79
       call     CORINFO_HELP_GETSHARED_NONGCSTATIC_BASE
       mov      rdi, ADDR
       mov      rsi, gword ptr [rbp-0xE0]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rsi, gword ptr [rbp-0xE0]
       mov      gword ptr [rbp-0xD8], rsi
 
G_M000_IG107:                ;; offset=0x1480
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0x120], rax
       mov      rdi, rax
       mov      rsi, gword ptr [rbp-0xD0]
       mov      rdx, gword ptr [rbp-0xD8]
       xor      rcx, rcx
       xor      r8d, r8d
       xor      r9, r9
       call     [System.Linq.OrderedEnumerable`2[double,double]:.ctor(System.Collections.Generic.IEnumerable`1[double],System.Func`2[double,double],System.Collections.Generic.IComparer`1[double],bool,System.Linq.OrderedEnumerable`1[double]):this]
       mov      rdi, gword ptr [rbp-0x120]
       call     [System.Linq.Enumerable:ToList[double](System.Collections.Generic.IEnumerable`1[double]):System.Collections.Generic.List`1[double]]
       mov      ecx, dword ptr [rax+0x10]
       mov      edx, ecx
       shr      edx, 31
       add      ecx, edx
       sar      ecx, 1
       cmp      ecx, dword ptr [rax+0x10]
       jae      G_M000_IG140
       mov      rdx, gword ptr [rax+0x08]
       cmp      ecx, dword ptr [rdx+0x08]
       jae      G_M000_IG157
       mov      edi, ecx
       vmovsd   xmm0, qword ptr [rdx+8*rdi+0x10]
       mov      r8, gword ptr [rbp+0x3B8]
       cmp      r12d, dword ptr [r8+0x08]
       jae      G_M000_IG157
       mov      edi, r12d
       mov      gword ptr [rbp+0x3B8], r8
       vmovsd   qword ptr [r8+8*rdi+0x10], xmm0
       inc      r12d
 
G_M000_IG108:                ;; offset=0x1511
       mov      rdi, gword ptr [rbp+0x450]
       cmp      dword ptr [rdi+0x08], r12d
       jg       G_M000_IG106
       mov      rdi, ADDR
       mov      r12, gword ptr [rdi]
       mov      rdi, ADDR
       mov      esi, 6
       call     CORINFO_HELP_NEWARR_1_OBJ
       mov      gword ptr [rbp-0xE8], rax
       lea      rdi, bword ptr [rax+0x10]
       mov      rsi, gword ptr [rbp+0x408]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      ecx, dword ptr [rbp+0x3F0]
       mov      dword ptr [rax+0x08], ecx
       mov      rdx, gword ptr [rbp-0xE8]
       lea      rdi, bword ptr [rdx+0x18]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      ecx, dword ptr [rbp+0x3D4]
       mov      dword ptr [rax+0x08], ecx
       mov      rdx, gword ptr [rbp-0xE8]
       lea      rdi, bword ptr [rdx+0x20]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0xF8], rax
       mov      rcx, gword ptr [rbp+0x448]
       mov      esi, dword ptr [rcx+0x08]
       xor      edi, edi
       call     [System.Linq.Enumerable:Range(int,int):System.Collections.Generic.IEnumerable`1[int]]
       mov      gword ptr [rbp-0x100], rax
       test     r13, r13
       je       G_M000_IG141
       mov      rcx, gword ptr [rbp-0xF8]
       lea      rdi, bword ptr [rcx+0x08]
       mov      rsi, r13
 
G_M000_IG109:                ;; offset=0x15F6
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdx, ADDR
       mov      r13, gword ptr [rbp-0xF8]
       mov      qword ptr [r13+0x18], rdx
       mov      rdx, r13
       mov      rsi, gword ptr [rbp-0x100]
       mov      rdi, ADDR
       call     [System.Linq.Enumerable:Select[int,System.__Canon](System.Collections.Generic.IEnumerable`1[int],System.Func`2[int,System.__Canon]):System.Collections.Generic.IEnumerable`1[System.__Canon]]
       mov      rsi, rax
       mov      rdi, ADDR
       call     [System.String:Join(System.String,System.Collections.Generic.IEnumerable`1[System.String]):System.String]
       mov      r13, gword ptr [rbp-0xE8]
       lea      rdi, bword ptr [r13+0x28]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       vmovsd   xmm0, qword ptr [rbp+0x3A0]
       vmovsd   qword ptr [rax+0x08], xmm0
       lea      rdi, bword ptr [r13+0x30]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, gword ptr [rbp+0x3B8]
       call     [System.Linq.Enumerable:MinFloat[double](System.Collections.Generic.IEnumerable`1[double]):double]
       mov      rdi, gword ptr [rbp+0x3B8]
       call     [System.Array:IndexOf[double](double[],double):int]
       mov      rcx, gword ptr [rbp+0x448]
       cmp      eax, dword ptr [rcx+0x08]
       jae      G_M000_IG157
       mov      edi, eax
       mov      gword ptr [rbp+0x448], rcx
       mov      rsi, gword ptr [rcx+8*rdi+0x10]
       lea      rdi, bword ptr [r13+0x38]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, r12
       mov      rdx, r13
       mov      rsi, ADDR
       call     [System.String:Format(System.IFormatProvider,System.String,System.Object[]):System.String]
       mov      r13, gword ptr [rbp+0x438]
       inc      dword ptr [r13+0x14]
       mov      r12, gword ptr [r13+0x08]
 
G_M000_IG110:                ;; offset=0x16DE
       mov      ecx, dword ptr [r13+0x10]
       cmp      dword ptr [r12+0x08], ecx
       jbe      SHORT G_M000_IG111
       lea      edi, [rcx+0x01]
       mov      dword ptr [r13+0x10], edi
       cmp      ecx, dword ptr [r12+0x08]
       jae      G_M000_IG157
       mov      edi, ecx
       lea      rdi, bword ptr [r12+8*rdi+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       jmp      SHORT G_M000_IG112
 
G_M000_IG111:                ;; offset=0x170C
       mov      rdi, r13
       mov      rsi, rax
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG112:                ;; offset=0x1718
       lea      rsi, [rbp+0x450]
       xor      edi, edi
       call     [Armonik.Ffi.Bdn.StrSweep:<Run>g__Set|2_0(int,byref)]
       mov      r12, ADDR
       xor      edi, edi
       mov      dword ptr [r12], edi
       mov      rax, ADDR
 
G_M000_IG113:                ;; offset=0x1741
       mov      qword ptr [rbp-0x70], rax
       mov      dword ptr [rax], edi
       mov      rcx, ADDR
       mov      qword ptr [rbp-0x78], rcx
       mov      byte  ptr [rcx], 0
       mov      esi, dword ptr [rbp+0x3FC]
       inc      esi
       mov      rdx, gword ptr [rbp+0x400]
       cmp      dword ptr [rdx+0x08], esi
       jg       G_M000_IG07
       jmp      G_M000_IG05
 
G_M000_IG114:                ;; offset=0x1775
       call     [System.Console:get_Error():System.IO.TextWriter]
       mov      gword ptr [rbp-0xB0], rax
       mov      rdi, ADDR
       mov      esi, 6
       call     CORINFO_HELP_NEWARR_1_OBJ
       mov      rdi, ADDR
       mov      gword ptr [rax+0x10], rdi
       mov      gword ptr [rbp-0xA8], rax
       lea      rdi, bword ptr [rax+0x18]
       mov      rsi, gword ptr [rbp+0x408]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      rax, gword ptr [rbp-0xA8]
       mov      gword ptr [rax+0x20], rdi
       mov      edi, dword ptr [rbp+0x3F0]
       call     [System.Number:Int32ToDecStr(int):System.String]
       mov      rcx, gword ptr [rbp-0xA8]
       lea      rdi, bword ptr [rcx+0x28]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      rax, gword ptr [rbp-0xA8]
       mov      gword ptr [rax+0x30], rdi
       mov      ecx, dword ptr [rbp+0x39C]
       mov      rdx, gword ptr [rbp+0x448]
       cmp      ecx, dword ptr [rdx+0x08]
       jae      G_M000_IG157
       mov      dword ptr [rbp+0x39C], ecx
       mov      edi, ecx
       mov      gword ptr [rbp+0x448], rdx
       mov      rsi, gword ptr [rdx+8*rdi+0x10]
       mov      gword ptr [rbp-0xA8], rax
       lea      rdi, bword ptr [rax+0x38]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, gword ptr [rbp-0xA8]
       call     [System.String:Concat(System.String[]):System.String]
       mov      rsi, rax
       mov      rdi, gword ptr [rbp-0xB0]
       mov      rax, qword ptr [rdi]
       mov      rax, qword ptr [rax+0x68]
       call     [rax+0x30]System.IO.TextWriter:WriteLine(System.String):this
       mov      eax, dword ptr [rbp+0x430]
       inc      eax
       mov      dword ptr [rbp+0x430], eax
 
G_M000_IG115:                ;; offset=0x186D
       jmp      G_M000_IG27
 
G_M000_IG116:                ;; offset=0x1872
       call     [Armonik.Ffi.Bdn.ProcCpu:Ns():long]
       mov      r12, rax
       call     [Armonik.Ffi.Bdn.ProcCpu:Wall():long]
       mov      rdi, r12
       sub      rdi, qword ptr [rbp+0x380]
       vxorps   xmm0, xmm0, xmm0
       vcvtsi2sd xmm0, xmm0, rdi
       mov      r12d, dword ptr [rbp+0x388]
       mov      rcx, gword ptr [rbp+0x3C8]
       cmp      r12d, dword ptr [rcx+0x08]
       jae      G_M000_IG157
       mov      edi, r12d
       vxorps   xmm1, xmm1, xmm1
       vcvtsi2sd xmm1, xmm1, qword ptr [rcx+8*rdi+0x10]
       vdivsd   xmm0, xmm0, xmm1
       mov      rdi, rax
       sub      rdi, qword ptr [rbp+0x378]
       vxorps   xmm1, xmm1, xmm1
       vcvtsi2sd xmm1, xmm1, rdi
       cmp      r12d, dword ptr [rcx+0x08]
       jae      G_M000_IG157
       mov      edi, r12d
       mov      gword ptr [rbp+0x3C8], rcx
       vxorps   xmm2, xmm2, xmm2
       vcvtsi2sd xmm2, xmm2, qword ptr [rcx+8*rdi+0x10]
       vdivsd   xmm1, xmm1, xmm2
       vmovsd   qword ptr [rbp+0x360], xmm1
       mov      rax, gword ptr [r13+0x10]
       cmp      r12d, dword ptr [rax+0x08]
       jae      G_M000_IG157
       mov      edi, r12d
       mov      rdi, gword ptr [rax+8*rdi+0x10]
       inc      dword ptr [rdi+0x14]
       mov      rsi, gword ptr [rdi+0x08]
       mov      r8d, dword ptr [rdi+0x10]
       cmp      dword ptr [rsi+0x08], r8d
       jbe      SHORT G_M000_IG117
       lea      eax, [r8+0x01]
       mov      dword ptr [rdi+0x10], eax
       cmp      r8d, dword ptr [rsi+0x08]
       jae      G_M000_IG157
       mov      edi, r8d
       vmovsd   qword ptr [rsi+8*rdi+0x10], xmm0
       vmovsd   qword ptr [rbp+0x368], xmm0
       jmp      SHORT G_M000_IG118
 
G_M000_IG117:                ;; offset=0x1947
       vmovsd   qword ptr [rbp+0x368], xmm0
       call     [System.Collections.Generic.List`1[double]:AddWithResize(double):this]
 
G_M000_IG118:                ;; offset=0x1955
       mov      rdi, ADDR
       mov      esi, 8
       call     CORINFO_HELP_NEWARR_1_OBJ
       mov      gword ptr [rbp-0x118], rax
       lea      rdi, bword ptr [rax+0x10]
       mov      rsi, gword ptr [rbp+0x408]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      ecx, dword ptr [rbp+0x3F0]
       mov      dword ptr [rax+0x08], ecx
       mov      rdx, gword ptr [rbp-0x118]
       lea      rdi, bword ptr [rdx+0x18]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      ecx, dword ptr [rbp+0x3D4]
       mov      dword ptr [rax+0x08], ecx
       mov      rdx, gword ptr [rbp-0x118]
       lea      rdi, bword ptr [rdx+0x20]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rax, gword ptr [rbp+0x448]
       cmp      r12d, dword ptr [rax+0x08]
       jae      G_M000_IG157
       mov      edi, r12d
       mov      gword ptr [rbp+0x448], rax
       mov      rsi, gword ptr [rax+8*rdi+0x10]
       mov      rcx, gword ptr [rbp-0x118]
       lea      rdi, bword ptr [rcx+0x28]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      ecx, dword ptr [rbp+0x390]
       lea      edi, [rcx+0x01]
       mov      dword ptr [rax+0x08], edi
       mov      rdx, gword ptr [rbp-0x118]
       lea      rdi, bword ptr [rdx+0x30]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
 
G_M000_IG119:                ;; offset=0x1A3E
       mov      rdi, gword ptr [rdi]
       vmovsd   xmm0, qword ptr [rbp+0x368]
       vmovsd   qword ptr [rbp-0x48], xmm0
       call     [System.Globalization.NumberFormatInfo:<GetInstance>g__GetProviderNonNull|58_0(System.IFormatProvider):System.Globalization.NumberFormatInfo]
       vmovsd   xmm0, qword ptr [rbp-0x48]
       mov      rsi, rax
       mov      rdi, ADDR
       call     [System.Number:FormatDouble(double,System.String,System.Globalization.NumberFormatInfo):System.String]
       mov      rcx, gword ptr [rbp-0x118]
       lea      rdi, bword ptr [rcx+0x38]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      rdi, gword ptr [rdi]
       vmovsd   xmm1, qword ptr [rbp+0x360]
       vmovsd   qword ptr [rbp-0x50], xmm1
       call     [System.Globalization.NumberFormatInfo:<GetInstance>g__GetProviderNonNull|58_0(System.IFormatProvider):System.Globalization.NumberFormatInfo]
       vmovsd   xmm0, qword ptr [rbp-0x50]
       mov      rsi, rax
       mov      rdi, ADDR
       call     [System.Number:FormatDouble(double,System.String,System.Globalization.NumberFormatInfo):System.String]
       mov      rcx, gword ptr [rbp-0x118]
       lea      rdi, bword ptr [rcx+0x40]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      rcx, gword ptr [rbp+0x3C8]
       cmp      r12d, dword ptr [rcx+0x08]
       jae      G_M000_IG157
       mov      edi, r12d
       mov      gword ptr [rbp+0x3C8], rcx
       mov      rdi, qword ptr [rcx+8*rdi+0x10]
       mov      qword ptr [rax+0x08], rdi
       mov      r12, gword ptr [rbp-0x118]
       lea      rdi, bword ptr [r12+0x48]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      esi, 1
       mov      rdx, r12
       call     [System.String:JoinCore(System.ReadOnlySpan`1[ushort],System.Object[]):System.String]
       mov      r12, gword ptr [rbp+0x440]
 
G_M000_IG120:                ;; offset=0x1B30
       inc      dword ptr [r12+0x14]
       mov      rdx, gword ptr [r12+0x08]
       mov      r8d, dword ptr [r12+0x10]
       cmp      dword ptr [rdx+0x08], r8d
       jbe      SHORT G_M000_IG121
       lea      edi, [r8+0x01]
       mov      dword ptr [r12+0x10], edi
       cmp      r8d, dword ptr [rdx+0x08]
       jae      G_M000_IG157
       mov      edi, r8d
       lea      rdi, bword ptr [rdx+8*rdi+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       jmp      SHORT G_M000_IG122
 
G_M000_IG121:                ;; offset=0x1B6A
       mov      rdi, r12
       mov      rsi, rax
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG122:                ;; offset=0x1B76
       mov      ecx, dword ptr [rbp+0x38C]
       inc      ecx
       mov      rax, gword ptr [rbp+0x450]
       cmp      dword ptr [rax+0x08], ecx
       jg       G_M000_IG82
       jmp      G_M000_IG81
 
G_M000_IG123:                ;; offset=0x1B93
       mov      eax, edi
       mov      r8d, eax
       jmp      SHORT G_M000_IG125
 
G_M000_IG124:                ;; offset=0x1B9A
       mov      r8d, 1
 
G_M000_IG125:                ;; offset=0x1BA0
       cmp      r8d, dword ptr [rcx+0x08]
       jae      G_M000_IG157
       mov      edi, r8d
       mov      rdi, qword ptr [rcx+8*rdi+0x10]
       cmp      rdi, 0x3E8
       jle      SHORT G_M000_IG126
       jmp      SHORT G_M000_IG127
 
G_M000_IG126:                ;; offset=0x1BBD
       mov      edi, 0x3E8
 
G_M000_IG127:                ;; offset=0x1BC2
       mov      qword ptr [rbp+0x3B0], rdi
       xor      edi, edi
       mov      dword ptr [rbp+0x354], edi
 
G_M000_IG128:                ;; offset=0x1BD1
       xor      edi, edi
       mov      qword ptr [rbp-0x58], rdi
       lea      rdi, [rbp-0x58]
       mov      rsi, gword ptr [rbp+0x3E0]
       mov      edx, 3
       call     [System.Runtime.InteropServices.GCHandle:.ctor(System.Object,int):this]
       mov      rdi, qword ptr [rbp-0x58]
       mov      qword ptr [rbp+0x348], rdi
       lea      rdi, [rbp+0x348]
       call     [System.Runtime.InteropServices.GCHandle:Free():this]
       mov      ecx, dword ptr [rbp+0x354]
       inc      ecx
       cmp      ecx, 3
       mov      dword ptr [rbp+0x354], ecx
       jl       SHORT G_M000_IG128
       call     [Armonik.Ffi.Bdn.ProcCpu:Ns():long]
       mov      qword ptr [rbp+0x3A8], rax
       xor      ecx, ecx
       jmp      SHORT G_M000_IG130
 
G_M000_IG129:                ;; offset=0x1C29
       xor      edi, edi
       mov      qword ptr [rbp-0x60], rdi
       lea      rdi, [rbp-0x60]
       mov      rsi, gword ptr [rbp+0x3E0]
       mov      edx, 3
       call     [System.Runtime.InteropServices.GCHandle:.ctor(System.Object,int):this]
       mov      rdi, qword ptr [rbp-0x60]
       mov      qword ptr [rbp+0x338], rdi
       lea      rdi, [rbp+0x338]
       call     [System.Runtime.InteropServices.GCHandle:Free():this]
       mov      rcx, qword ptr [rbp+0x340]
       inc      rcx
 
G_M000_IG130:                ;; offset=0x1C67
       mov      qword ptr [rbp+0x340], rcx
       cmp      rcx, qword ptr [rbp+0x3B0]
       jl       SHORT G_M000_IG129
       call     [Armonik.Ffi.Bdn.ProcCpu:Ns():long]
       sub      rax, qword ptr [rbp+0x3A8]
       vxorps   xmm0, xmm0, xmm0
       vcvtsi2sd xmm0, xmm0, rax
       vxorps   xmm1, xmm1, xmm1
       vcvtsi2sd xmm1, xmm1, qword ptr [rbp+0x3B0]
       vdivsd   xmm0, xmm0, xmm1
       vmovsd   qword ptr [rbp+0x3A0], xmm0
       xor      edi, edi
       xor      eax, eax
       jmp      G_M000_IG134
 
G_M000_IG131:                ;; offset=0x1CAF
       mov      rdi, gword ptr [r13+0x10]
       cmp      eax, dword ptr [rdi+0x08]
       jae      G_M000_IG157
       mov      dword ptr [rbp+0x334], eax
       mov      esi, eax
       mov      rcx, gword ptr [rdi+8*rsi+0x10]
       mov      gword ptr [rbp-0xC8], rcx
       mov      rdi, ADDR
       mov      esi, 79
       call     CORINFO_HELP_GETSHARED_NONGCSTATIC_BASE
       mov      rdi, ADDR
       mov      rax, gword ptr [rdi]
       mov      rdx, rax
       mov      rsi, gword ptr [rbp-0xC8]
       mov      gword ptr [rbp-0xD0], rsi
       mov      rcx, rdx
       test     rcx, rcx
       mov      gword ptr [rbp-0xD8], rcx
       jne      G_M000_IG133
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0xE0], rax
       mov      rdi, ADDR
       mov      esi, 79
       call     CORINFO_HELP_GETSHARED_NONGCSTATIC_BASE
       mov      rdi, ADDR
       mov      rsi, gword ptr [rdi]
       test     rsi, rsi
       mov      gword ptr [rbp+0x440], r12
       je       G_M000_IG141
       mov      r12d, dword ptr [rbp+0x334]
       mov      rcx, gword ptr [rbp-0xE0]
       lea      rdi, bword ptr [rcx+0x08]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      rcx, gword ptr [rbp-0xE0]
       mov      qword ptr [rcx+0x18], rdi
       mov      rdi, ADDR
       mov      esi, 79
       call     CORINFO_HELP_GETSHARED_NONGCSTATIC_BASE
       mov      rdi, ADDR
       mov      rsi, gword ptr [rbp-0xE0]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rcx, gword ptr [rbp-0xE0]
       mov      gword ptr [rbp-0xD8], rcx
 
G_M000_IG132:                ;; offset=0x1DC0
       mov      dword ptr [rbp+0x334], r12d
       mov      r12, gword ptr [rbp+0x440]
 
G_M000_IG133:                ;; offset=0x1DCE
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0x120], rax
       mov      rdi, rax
       mov      rsi, gword ptr [rbp-0xD0]
       mov      rdx, gword ptr [rbp-0xD8]
       xor      rcx, rcx
       xor      r8d, r8d
       xor      r9, r9
       call     [System.Linq.OrderedEnumerable`2[double,double]:.ctor(System.Collections.Generic.IEnumerable`1[double],System.Func`2[double,double],System.Collections.Generic.IComparer`1[double],bool,System.Linq.OrderedEnumerable`1[double]):this]
       mov      rdi, gword ptr [rbp-0x120]
       call     [System.Linq.Enumerable:ToList[double](System.Collections.Generic.IEnumerable`1[double]):System.Collections.Generic.List`1[double]]
       mov      ecx, dword ptr [rax+0x10]
       mov      edi, ecx
       shr      edi, 31
       add      ecx, edi
       sar      ecx, 1
       cmp      ecx, dword ptr [rax+0x10]
       mov      gword ptr [rbp+0x440], r12
       jae      G_M000_IG140
       mov      r12d, dword ptr [rbp+0x334]
       mov      rdx, gword ptr [rax+0x08]
       cmp      ecx, dword ptr [rdx+0x08]
       jae      G_M000_IG157
       mov      edi, ecx
       vmovsd   xmm0, qword ptr [rdx+8*rdi+0x10]
       mov      rax, gword ptr [rbp+0x3B8]
       cmp      r12d, dword ptr [rax+0x08]
       jae      G_M000_IG157
       mov      edi, r12d
       mov      gword ptr [rbp+0x3B8], rax
       vmovsd   qword ptr [rax+8*rdi+0x10], xmm0
       inc      r12d
       mov      eax, r12d
       mov      r12, gword ptr [rbp+0x440]
 
G_M000_IG134:                ;; offset=0x1E76
       mov      rdi, gword ptr [rbp+0x450]
       cmp      dword ptr [rdi+0x08], eax
       jg       G_M000_IG131
       mov      rdi, ADDR
       mov      rdi, gword ptr [rdi]
       mov      gword ptr [rbp-0xF0], rdi
       mov      rdi, ADDR
       mov      esi, 6
       call     CORINFO_HELP_NEWARR_1_OBJ
       mov      gword ptr [rbp-0xE8], rax
       lea      rdi, bword ptr [rax+0x10]
       mov      rsi, gword ptr [rbp+0x408]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      ecx, dword ptr [rbp+0x3F0]
       mov      dword ptr [rax+0x08], ecx
       mov      rcx, gword ptr [rbp-0xE8]
       lea      rdi, bword ptr [rcx+0x18]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      ecx, dword ptr [rbp+0x3D4]
       mov      dword ptr [rax+0x08], ecx
       mov      rcx, gword ptr [rbp-0xE8]
       lea      rdi, bword ptr [rcx+0x20]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0xF8], rax
       mov      rcx, gword ptr [rbp+0x448]
       mov      esi, dword ptr [rcx+0x08]
       xor      edi, edi
       call     [System.Linq.Enumerable:Range(int,int):System.Collections.Generic.IEnumerable`1[int]]
       test     r13, r13
       mov      gword ptr [rbp+0x440], r12
       mov      gword ptr [rbp-0x100], rax
       je       G_M000_IG141
       mov      rdx, gword ptr [rbp-0xF8]
       mov      gword ptr [rbp-0xF8], rdx
       lea      rdi, bword ptr [rdx+0x08]
 
G_M000_IG135:                ;; offset=0x1F6C
       mov      rsi, r13
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdx, ADDR
       mov      r13, gword ptr [rbp-0xF8]
       mov      qword ptr [r13+0x18], rdx
       mov      rdx, r13
       mov      rsi, gword ptr [rbp-0x100]
       mov      rdi, ADDR
       call     [System.Linq.Enumerable:Select[int,System.__Canon](System.Collections.Generic.IEnumerable`1[int],System.Func`2[int,System.__Canon]):System.Collections.Generic.IEnumerable`1[System.__Canon]]
       mov      rsi, rax
       mov      rdi, ADDR
       call     [System.String:Join(System.String,System.Collections.Generic.IEnumerable`1[System.String]):System.String]
       mov      r13, gword ptr [rbp-0xE8]
       lea      rdi, bword ptr [r13+0x28]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       vmovsd   xmm0, qword ptr [rbp+0x3A0]
       vmovsd   qword ptr [rax+0x08], xmm0
       lea      rdi, bword ptr [r13+0x30]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, gword ptr [rbp+0x3B8]
       call     [System.Linq.Enumerable:MinFloat[double](System.Collections.Generic.IEnumerable`1[double]):double]
       mov      rdi, gword ptr [rbp+0x3B8]
       call     [System.Array:IndexOf[double](double[],double):int]
       mov      rcx, gword ptr [rbp+0x448]
       cmp      eax, dword ptr [rcx+0x08]
       jae      G_M000_IG157
       mov      edi, eax
       mov      gword ptr [rbp+0x448], rcx
       mov      rsi, gword ptr [rcx+8*rdi+0x10]
       lea      rdi, bword ptr [r13+0x38]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, gword ptr [rbp-0xF0]
       mov      rdx, r13
       mov      rsi, ADDR
       call     [System.String:Format(System.IFormatProvider,System.String,System.Object[]):System.String]
       mov      r13, gword ptr [rbp+0x438]
       inc      dword ptr [r13+0x14]
 
G_M000_IG136:                ;; offset=0x2057
       mov      rdi, gword ptr [r13+0x08]
       mov      ecx, dword ptr [r13+0x10]
       cmp      dword ptr [rdi+0x08], ecx
       jbe      SHORT G_M000_IG137
       lea      esi, [rcx+0x01]
       mov      dword ptr [r13+0x10], esi
       cmp      ecx, dword ptr [rdi+0x08]
       jae      G_M000_IG157
       mov      esi, ecx
       lea      rdi, bword ptr [rdi+8*rsi+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       jmp      SHORT G_M000_IG138
 
G_M000_IG137:                ;; offset=0x2085
       mov      rdi, r13
       mov      rsi, rax
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG138:                ;; offset=0x2091
       lea      rsi, [rbp+0x450]
       xor      edi, edi
       call     [Armonik.Ffi.Bdn.StrSweep:<Run>g__Set|2_0(int,byref)]
       mov      rdi, ADDR
       mov      rax, rdi
       xor      edi, edi
       mov      qword ptr [rbp-0x68], rax
       mov      dword ptr [rax], edi
       mov      rdi, ADDR
       mov      rcx, rdi
       xor      edi, edi
       mov      qword ptr [rbp-0x70], rcx
       mov      dword ptr [rcx], edi
       mov      rdi, ADDR
       mov      rdx, rdi
       mov      qword ptr [rbp-0x78], rdx
       mov      byte  ptr [rdx], 0
       mov      esi, dword ptr [rbp+0x3FC]
       inc      esi
       mov      r8, gword ptr [rbp+0x400]
       cmp      dword ptr [r8+0x08], esi
       jg       G_M000_IG46
       jmp      G_M000_IG44
 
G_M000_IG139:                ;; offset=0x20FC
       call     [System.IO.File:get_UTF8NoBOM():System.Text.Encoding]
       mov      rdx, rax
       mov      rsi, gword ptr [rbp+0x440]
       mov      rdi, r14
       call     [System.IO.File:WriteAllLines(System.String,System.Collections.Generic.IEnumerable`1[System.String],System.Text.Encoding)]
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      r12, rax
       mov      rdi, r12
       call     [System.Collections.Generic.List`1[System.__Canon]:.ctor():this]
       mov      edi, ebx
       call     [System.Number:Int32ToDecStr(int):System.String]
       mov      rsi, rax
       mov      rdi, ADDR
       mov      rdx, ADDR
       call     [System.String:Concat(System.String,System.String,System.String):System.String]
       inc      dword ptr [r12+0x14]
       mov      rdi, gword ptr [r12+0x08]
       mov      esi, dword ptr [r12+0x10]
       cmp      dword ptr [rdi+0x08], esi
       jbe      SHORT G_M000_IG142
       lea      ecx, [rsi+0x01]
       mov      dword ptr [r12+0x10], ecx
       cmp      esi, dword ptr [rdi+0x08]
       jae      G_M000_IG157
       lea      rdi, bword ptr [rdi+8*rsi+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       jmp      SHORT G_M000_IG143
 
G_M000_IG140:                ;; offset=0x2189
       call     [System.ThrowHelper:ThrowArgumentOutOfRange_IndexMustBeLessException()]
       int3     
 
G_M000_IG141:                ;; offset=0x2190
       call     [System.MulticastDelegate:ThrowNullThisInDelegateToInstance()]
       int3     
 
G_M000_IG142:                ;; offset=0x2197
       mov      rdi, r12
       mov      rsi, rax
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG143:                ;; offset=0x21A3
       inc      dword ptr [r12+0x14]
       mov      rdi, gword ptr [r12+0x08]
       mov      esi, dword ptr [r12+0x10]
       cmp      dword ptr [rdi+0x08], esi
       jbe      SHORT G_M000_IG144
       lea      eax, [rsi+0x01]
       mov      dword ptr [r12+0x10], eax
       cmp      esi, dword ptr [rdi+0x08]
       jae      G_M000_IG157
       mov      rax, ADDR
       mov      gword ptr [rdi+8*rsi+0x10], rax
       jmp      SHORT G_M000_IG145
 
G_M000_IG144:                ;; offset=0x21D9
       mov      rdi, r12
       mov      rsi, ADDR
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG145:                ;; offset=0x21EC
       mov      rdi, ADDR
       mov      esi, 5
       call     CORINFO_HELP_NEWARR_1_OBJ
       mov      rbx, rax
       mov      rax, ADDR
       mov      gword ptr [rbx+0x10], rax
       call     [Armonik.Ffi.Bdn.StrSweep:Env():System.String]
       lea      rdi, bword ptr [rbx+0x18]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      gword ptr [rbx+0x20], rdi
       mov      r15d, dword ptr [rbp+0x430]
       test     r15d, r15d
       je       SHORT G_M000_IG146
       mov      gword ptr [rbp-0x108], rbx
       mov      edi, r15d
       call     [System.Number:Int32ToDecStr(int):System.String]
       mov      rdi, rax
       mov      rsi, ADDR
       call     [System.String:Concat(System.String,System.String):System.String]
       jmp      SHORT G_M000_IG147
 
G_M000_IG146:                ;; offset=0x2262
       mov      rdi, rbx
       mov      rax, ADDR
       mov      gword ptr [rbp-0x108], rdi
 
G_M000_IG147:                ;; offset=0x2276
       mov      esi, 3
       mov      rdi, gword ptr [rbp-0x108]
       mov      rdx, rax
       call     CORINFO_HELP_ARRADDR_ST
       mov      rdi, rbx
       mov      esi, 4
       mov      rdx, ADDR
       call     CORINFO_HELP_ARRADDR_ST
       mov      rdi, rbx
       call     [System.String:Concat(System.String[]):System.String]
       inc      dword ptr [r12+0x14]
       mov      rdi, gword ptr [r12+0x08]
       mov      esi, dword ptr [r12+0x10]
       cmp      dword ptr [rdi+0x08], esi
       jbe      SHORT G_M000_IG148
       lea      ecx, [rsi+0x01]
       mov      dword ptr [r12+0x10], ecx
       cmp      esi, dword ptr [rdi+0x08]
       jae      G_M000_IG157
       lea      rdi, bword ptr [rdi+8*rsi+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       jmp      SHORT G_M000_IG149
 
G_M000_IG148:                ;; offset=0x22DE
       mov      rdi, r12
       mov      rsi, rax
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG149:                ;; offset=0x22EA
       inc      dword ptr [r12+0x14]
       mov      rdi, gword ptr [r12+0x08]
       mov      esi, dword ptr [r12+0x10]
       cmp      dword ptr [rdi+0x08], esi
       jbe      SHORT G_M000_IG150
       lea      eax, [rsi+0x01]
       mov      dword ptr [r12+0x10], eax
       cmp      esi, dword ptr [rdi+0x08]
       jae      G_M000_IG157
       mov      rax, ADDR
       mov      gword ptr [rdi+8*rsi+0x10], rax
       jmp      SHORT G_M000_IG151
 
G_M000_IG150:                ;; offset=0x2320
       mov      rdi, r12
       mov      rsi, ADDR
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG151:                ;; offset=0x2333
       mov      rsi, gword ptr [rbp+0x448]
       mov      rdi, ADDR
       call     [System.String:Join(System.String,System.String[]):System.String]
       mov      rsi, rax
       mov      rdi, ADDR
       mov      rdx, ADDR
       call     [System.String:Concat(System.String,System.String,System.String):System.String]
       inc      dword ptr [r12+0x14]
       mov      rdi, gword ptr [r12+0x08]
       mov      esi, dword ptr [r12+0x10]
       cmp      dword ptr [rdi+0x08], esi
       jbe      SHORT G_M000_IG152
       lea      ecx, [rsi+0x01]
       mov      dword ptr [r12+0x10], ecx
       cmp      esi, dword ptr [rdi+0x08]
       jae      G_M000_IG157
       lea      rdi, bword ptr [rdi+8*rsi+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       jmp      SHORT G_M000_IG153
 
G_M000_IG152:                ;; offset=0x239B
       mov      rdi, r12
       mov      rsi, rax
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG153:                ;; offset=0x23A7
       mov      rdi, ADDR
       mov      esi, 79
       call     CORINFO_HELP_GETSHARED_NONGCSTATIC_BASE
       mov      rdi, ADDR
       mov      rdx, gword ptr [rdi]
       mov      rbx, gword ptr [rbp+0x448]
       test     rdx, rdx
       jne      G_M000_IG154
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0x110], rax
       mov      rdi, ADDR
       mov      esi, 79
       call     CORINFO_HELP_GETSHARED_NONGCSTATIC_BASE
       mov      rsi, ADDR
       mov      rsi, gword ptr [rsi]
       mov      rdi, gword ptr [rbp-0x110]
       mov      rdx, ADDR
       call     [System.MulticastDelegate:CtorClosed(System.Object,long):this]
       mov      rdi, ADDR
       mov      esi, 79
       call     CORINFO_HELP_GETSHARED_NONGCSTATIC_BASE
       mov      rdi, ADDR
       mov      rsi, gword ptr [rbp-0x110]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdx, gword ptr [rbp-0x110]
 
G_M000_IG154:                ;; offset=0x2457
       mov      rsi, rbx
       mov      rdi, ADDR
       call     [System.Linq.Enumerable:Select[System.__Canon,System.__Canon](System.Collections.Generic.IEnumerable`1[System.__Canon],System.Func`2[System.__Canon,System.__Canon]):System.Collections.Generic.IEnumerable`1[System.__Canon]]
       mov      rdi, rax
       call     [System.String:Concat(System.Collections.Generic.IEnumerable`1[System.String]):System.String]
       mov      rsi, rax
       mov      rdi, ADDR
       mov      rdx, ADDR
       call     [System.String:Concat(System.String,System.String,System.String):System.String]
       inc      dword ptr [r12+0x14]
       mov      rdi, gword ptr [r12+0x08]
       mov      esi, dword ptr [r12+0x10]
       cmp      dword ptr [rdi+0x08], esi
       jbe      SHORT G_M000_IG155
       lea      ecx, [rsi+0x01]
       mov      dword ptr [r12+0x10], ecx
       cmp      esi, dword ptr [rdi+0x08]
       jae      SHORT G_M000_IG157
       lea      rdi, bword ptr [rdi+8*rsi+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       jmp      SHORT G_M000_IG156
 
G_M000_IG155:                ;; offset=0x24C0
       mov      rdi, r12
       mov      rsi, rax
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG156:                ;; offset=0x24CC
       mov      rdi, r12
       mov      rsi, r13
       call     [System.Collections.Generic.List`1[System.__Canon]:AddRange(System.Collections.Generic.IEnumerable`1[System.__Canon]):this]
       mov      rdi, r14
       mov      rsi, ADDR
       call     [System.IO.Path:ChangeExtension(System.String,System.String):System.String]
       mov      r13, rax
       call     [System.IO.File:get_UTF8NoBOM():System.Text.Encoding]
       mov      rdx, rax
       mov      rdi, r13
       mov      rsi, r12
       call     [System.IO.File:WriteAllLines(System.String,System.Collections.Generic.IEnumerable`1[System.String],System.Text.Encoding)]
       lea      rsi, [rbp+0x308]
       mov      rdi, r12
       call     [System.Collections.Generic.List`1[System.__Canon]:GetEnumerator():System.Collections.Generic.List`1+Enumerator[System.__Canon]:this]
       jmp      G_M000_IG87
 
G_M000_IG157:                ;; offset=0x2518
       call     CORINFO_HELP_RNGCHKFAIL
       int3     
 
RWD00  	dq	412E848000000000h	;      1000000
RWD08  	dq	408F400000000000h	;         1000
RWD16  	dq	3FD0000000000000h	;         0.25
RWD24  	dd	00000000h, 00000000h
RWD32  	dq	3F50624DD2F1A9FCh, 0000000000000000h

; Total bytes of code 9502

