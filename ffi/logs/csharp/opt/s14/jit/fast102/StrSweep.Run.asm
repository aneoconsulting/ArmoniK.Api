; Assembly listing for method Armonik.Ffi.Bdn.StrSweep:Run(System.String,int,double,int[],System.String[],System.String[]):int (Tier1-OSR)
; Emitting BLENDED_CODE for X64 with AVX512 - Unix
; Tier1-OSR code
; OSR variant for entry point 0x2db
; optimized code
; optimized using Dynamic PGO
; rbp based frame
; partially interruptible
; with Dynamic PGO: edge weights are invalid, and fgCalledCount is 99.23
; 0 inlinees with PGO data; 35 single block inlinees; 21 inlinees without PGO data

G_M000_IG01:                ;; offset=0x0000
       mov      rax, qword ptr [rbp]
       push     rax
       sub      rsp, 256
       mov      qword ptr [rsp+0x5C8], r15
       mov      qword ptr [rsp+0x5C0], r14
       mov      qword ptr [rsp+0x5B8], r13
       mov      qword ptr [rsp+0x5B0], r12
       mov      qword ptr [rsp+0x5A8], rbx
       vzeroupper 
       lea      rbp, [rsp+0x100]
       mov      r14, gword ptr [rbp+0x4A0]
       mov      ebx, dword ptr [rbp+0x49C]
       mov      r15, gword ptr [rbp+0x488]
       mov      r10, gword ptr [rbp+0x418]
       mov      r8d, dword ptr [rbp+0x414]
       mov      rax, gword ptr [rbp+0x3E8]
       mov      r12, gword ptr [rbp+0x3C8]
       mov      ecx, dword ptr [rbp+0x388]
       mov      r13, qword ptr [rbp+0x358]
 
G_M000_IG02:                ;; offset=0x007C
       mov      gword ptr [rbp+0x418], r10
       test     r10, r10
       je       G_M000_IG59
       mov      dword ptr [rbp+0x414], r8d
       test     r8d, r8d
       jl       SHORT G_M000_IG03
       jmp      G_M000_IG28
 
G_M000_IG03:                ;; offset=0x009D
       mov      r8d, dword ptr [rbp+0x414]
       jmp      G_M000_IG59
 
G_M000_IG04:                ;; offset=0x00A9
       mov      ecx, r8d
       mov      rsi, gword ptr [rax+8*rcx+0x10]
       mov      rcx, rsi
       mov      rdx, r15
       xor      esi, esi
       xor      edi, edi
       cmp      dword ptr [r15+0x08], 0
       mov      dword ptr [rbp+0x414], r8d
       mov      gword ptr [rbp+0x418], rax
       mov      gword ptr [rbp+0x408], rcx
       mov      esi, edi
       jg       SHORT G_M000_IG06
 
G_M000_IG05:                ;; offset=0x00D9
       mov      r8d, dword ptr [rbp+0x414]
       inc      r8d
       mov      rax, gword ptr [rbp+0x418]
       mov      ecx, dword ptr [rax+0x08]
       cmp      ecx, r8d
       jle      G_M000_IG120
       jmp      SHORT G_M000_IG04
 
G_M000_IG06:                ;; offset=0x00F8
       mov      rax, rdx
 
G_M000_IG07:                ;; offset=0x00FB
       cmp      esi, dword ptr [rax+0x08]
       jae      G_M000_IG138
       mov      dword ptr [rbp+0x3FC], esi
       mov      edi, esi
       mov      gword ptr [rbp+0x400], rax
       mov      ecx, dword ptr [rax+4*rdi+0x10]
       mov      dword ptr [rbp+0x3F0], ecx
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp+0x3E8], rax
       mov      rdi, gword ptr [rbp+0x408]
       mov      esi, dword ptr [rbp+0x3F0]
       call     [Armonik.Ffi.Bdn.StrSweep:Make(System.String,int):System.String]
       mov      gword ptr [rbp+0x3E0], rax
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0x60], rax
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0x68], rax
       mov      rdi, rax
       call     [Armonik.Ffi.Facade.UploadResultData:.ctor():this]
       mov      rax, gword ptr [rbp-0x68]
       lea      rdi, bword ptr [rax+0x08]
       mov      rsi, gword ptr [rbp+0x3E0]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rax, gword ptr [rbp-0x60]
       lea      rdi, bword ptr [rax+0x08]
       mov      rsi, gword ptr [rbp-0x68]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rax, gword ptr [rbp+0x3E8]
       lea      rdi, bword ptr [rax+0x08]
       mov      rsi, gword ptr [rbp-0x60]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0x70], rax
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0x78], rax
 
G_M000_IG08:                ;; offset=0x01DB
       mov      rdi, rax
       call     [Armonik.Ffi.Shapes.V1.UploadResultData:.ctor():this]
       mov      rdi, gword ptr [rbp-0x78]
       mov      rsi, gword ptr [rbp+0x3E0]
       call     [Armonik.Ffi.Shapes.V1.UploadResultData:set_SessionId(System.String):this]
       mov      rax, gword ptr [rbp-0x70]
       lea      rdi, bword ptr [rax+0x10]
       mov      rsi, gword ptr [rbp-0x78]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, gword ptr [rbp-0x70]
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
 
G_M000_IG09:                ;; offset=0x024C
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
       jl       G_M000_IG39
       mov      rdi, gword ptr [rbp+0x468]
       cmp      dword ptr [rdi], edi
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:ContextBytes():ubyte[]:this]
       test     rax, rax
       jne      SHORT G_M000_IG11
 
G_M000_IG10:                ;; offset=0x0299
       xor      rdi, rdi
       xor      ecx, ecx
       jmp      SHORT G_M000_IG12
 
G_M000_IG11:                ;; offset=0x029F
       lea      rdi, bword ptr [rax+0x10]
       mov      ecx, dword ptr [rax+0x08]
 
G_M000_IG12:                ;; offset=0x02A6
       mov      r8, gword ptr [rbp+0x3D8]
       test     r8, r8
       jne      SHORT G_M000_IG14
 
G_M000_IG13:                ;; offset=0x02B2
       xor      rsi, rsi
       xor      r9d, r9d
       jmp      SHORT G_M000_IG15
 
G_M000_IG14:                ;; offset=0x02B9
       lea      rsi, bword ptr [r8+0x10]
       mov      gword ptr [rbp+0x3D8], r8
       mov      r9d, dword ptr [r8+0x08]
       mov      r8, gword ptr [rbp+0x3D8]
 
G_M000_IG15:                ;; offset=0x02CF
       cmp      ecx, r9d
       mov      gword ptr [rbp+0x3D8], r8
       jne      G_M000_IG70
 
G_M000_IG16:                ;; offset=0x02DF
       mov      edx, r9d
       call     [System.SpanHelpers:SequenceEqual(byref,byref,ulong):bool]
       test     eax, eax
       je       G_M000_IG70
 
G_M000_IG17:                ;; offset=0x02F0
       mov      ecx, dword ptr [rbp+0x39C]
       inc      ecx
       mov      rsi, gword ptr [rbp+0x450]
       cmp      dword ptr [rsi+0x08], ecx
       jg       G_M000_IG09
 
G_M000_IG18:                ;; offset=0x0308
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
       jbe      G_M000_IG22
 
G_M000_IG19:                ;; offset=0x0365
       xor      eax, eax
       mov      rdx, gword ptr [rbp+0x450]
       cmp      dword ptr [rdx+0x08], 0
       jle      SHORT G_M000_IG21
 
G_M000_IG20:                ;; offset=0x0374
       lea      rdx, [rbp+0x450]
       mov      rdi, gword ptr [rbp+0x3E8]
       mov      dword ptr [rbp+0x398], eax
       mov      esi, eax
       call     [Armonik.Ffi.Bdn.StrSweep+<>c__DisplayClass2_1:<Run>g__Iters|1(int,byref):long:this]
       mov      ecx, dword ptr [rbp+0x398]
       mov      rdx, gword ptr [rbp+0x3C8]
       cmp      ecx, dword ptr [rdx+0x08]
       jae      G_M000_IG138
       mov      edi, ecx
       mov      gword ptr [rbp+0x3C8], rdx
       mov      qword ptr [rdx+8*rdi+0x10], rax
       inc      ecx
       mov      rax, gword ptr [rbp+0x450]
       cmp      dword ptr [rax+0x08], ecx
       mov      eax, ecx
       jg       SHORT G_M000_IG20
 
G_M000_IG21:                ;; offset=0x03C4
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
 
G_M000_IG22:                ;; offset=0x03FE
       mov      rax, gword ptr [rbp+0x450]
       mov      gword ptr [rbp-0xF8], rax
       mov      esi, dword ptr [rax+0x08]
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWARR_1_VC
       mov      gword ptr [rbp+0x3B8], rax
       mov      rsi, gword ptr [rbp-0xF8]
       mov      esi, dword ptr [rsi+0x08]
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWARR_1_OBJ
       mov      rcx, gword ptr [rbp+0x3E8]
       lea      rdi, bword ptr [rcx+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       xor      eax, eax
       mov      dword ptr [rbp+0x394], eax
       mov      rdi, gword ptr [rbp+0x450]
       cmp      dword ptr [rdi+0x08], 0
       jle      SHORT G_M000_IG24
 
G_M000_IG23:                ;; offset=0x0466
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      rcx, gword ptr [rbp+0x3E8]
       mov      rdx, gword ptr [rcx+0x10]
       mov      gword ptr [rbp-0x98], rdx
       mov      gword ptr [rbp-0x90], rax
       mov      rdi, rax
       call     [System.Collections.Generic.List`1[double]:.ctor():this]
       movsxd   rsi, dword ptr [rbp+0x394]
       mov      rdi, gword ptr [rbp-0x98]
       mov      rdx, gword ptr [rbp-0x90]
       call     CORINFO_HELP_ARRADDR_ST
       mov      eax, dword ptr [rbp+0x394]
       inc      eax
       mov      rcx, gword ptr [rbp+0x450]
       cmp      dword ptr [rcx+0x08], eax
       mov      dword ptr [rbp+0x394], eax
       jg       SHORT G_M000_IG23
 
G_M000_IG24:                ;; offset=0x04CB
       xor      r8d, r8d
       xor      ecx, ecx
       test     ebx, ebx
       mov      gword ptr [rbp+0x438], r12
       mov      r8d, ecx
       mov      r12, gword ptr [rbp+0x3C8]
       jle      G_M000_IG79
 
G_M000_IG25:                ;; offset=0x04E9
       xor      ecx, ecx
       xor      eax, eax
       mov      rdi, gword ptr [rbp+0x450]
       cmp      dword ptr [rdi+0x08], 0
       mov      dword ptr [rbp+0x390], r8d
       mov      ecx, eax
       jg       SHORT G_M000_IG27
 
G_M000_IG26:                ;; offset=0x0503
       mov      r8d, dword ptr [rbp+0x390]
       inc      r8d
       cmp      r8d, ebx
       jge      G_M000_IG79
       jmp      SHORT G_M000_IG25
 
G_M000_IG27:                ;; offset=0x0518
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
       mov      gword ptr [rbp+0x440], r13
       mov      r13, rcx
       mov      rax, gword ptr [rbp+0x3E8]
       mov      ecx, dword ptr [rbp+0x388]
 
G_M000_IG28:                ;; offset=0x057E
       mov      esi, dword ptr [r12+0x08]
       cmp      ecx, esi
       jae      G_M000_IG138
       mov      dword ptr [rbp+0x388], ecx
       mov      esi, ecx
       mov      qword ptr [rbp-0x58], rsi
       cmp      qword ptr [r12+8*rsi+0x10], r13
       jle      SHORT G_M000_IG31
 
G_M000_IG29:                ;; offset=0x059E
       mov      gword ptr [rbp+0x3E8], rax
       mov      rsi, gword ptr [rax+0x08]
       mov      rdi, gword ptr [rbp+0x468]
       xor      edx, edx
       cmp      dword ptr [rdi], edi
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:EncodeInto(Armonik.Ffi.Facade.UploadResultDataMessage,bool):int:this]
       inc      r13
       mov      rax, qword ptr [rbp-0x58]
       cmp      qword ptr [r12+8*rax+0x10], r13
       jle      SHORT G_M000_IG30
       mov      rax, gword ptr [rbp+0x3E8]
       jmp      SHORT G_M000_IG29
 
G_M000_IG30:                ;; offset=0x05D1
       jmp      G_M000_IG72
 
G_M000_IG31:                ;; offset=0x05D6
       mov      gword ptr [rbp+0x3E8], rax
       jmp      G_M000_IG72
 
G_M000_IG32:                ;; offset=0x05E2
       cmp      r8d, edx
       jae      G_M000_IG138
       mov      ecx, r8d
       mov      rsi, gword ptr [rax+8*rcx+0x10]
       mov      rcx, r15
       xor      edx, edx
       cmp      dword ptr [r15+0x08], 0
       mov      dword ptr [rbp+0x414], r8d
       mov      gword ptr [rbp+0x418], rax
       mov      gword ptr [rbp+0x408], rsi
       jg       SHORT G_M000_IG34
 
G_M000_IG33:                ;; offset=0x0614
       mov      r8d, dword ptr [rbp+0x414]
       inc      r8d
       mov      rcx, gword ptr [rbp+0x418]
       mov      edx, dword ptr [rcx+0x08]
       cmp      edx, r8d
       jle      G_M000_IG120
       mov      rax, rcx
       jmp      SHORT G_M000_IG32
 
G_M000_IG34:                ;; offset=0x0636
       mov      eax, edx
 
G_M000_IG35:                ;; offset=0x0638
       cmp      eax, dword ptr [rcx+0x08]
       jae      G_M000_IG138
       mov      dword ptr [rbp+0x3FC], eax
       mov      edi, eax
       mov      gword ptr [rbp+0x400], rcx
       mov      edi, dword ptr [rcx+4*rdi+0x10]
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
       mov      gword ptr [rbp-0x60], rax
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0x68], rax
       mov      rdi, rax
       call     [Armonik.Ffi.Facade.UploadResultData:.ctor():this]
       mov      rax, gword ptr [rbp-0x68]
       lea      rdi, bword ptr [rax+0x08]
       mov      rsi, gword ptr [rbp+0x3E0]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rax, gword ptr [rbp-0x60]
       lea      rdi, bword ptr [rax+0x08]
       mov      rsi, gword ptr [rbp-0x68]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rax, gword ptr [rbp+0x3E8]
       lea      rdi, bword ptr [rax+0x08]
       mov      rsi, gword ptr [rbp-0x60]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0x70], rax
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0x78], rax
 
G_M000_IG36:                ;; offset=0x0718
       mov      rdi, rax
       call     [Armonik.Ffi.Shapes.V1.UploadResultData:.ctor():this]
       mov      rdi, gword ptr [rbp-0x78]
       mov      rsi, gword ptr [rbp+0x3E0]
       call     [Armonik.Ffi.Shapes.V1.UploadResultData:set_SessionId(System.String):this]
       mov      rax, gword ptr [rbp-0x70]
       lea      rdi, bword ptr [rax+0x10]
       mov      rsi, gword ptr [rbp-0x78]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, gword ptr [rbp-0x70]
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
       jg       SHORT G_M000_IG38
 
G_M000_IG37:                ;; offset=0x0787
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
       jbe      G_M000_IG53
       jmp      G_M000_IG51
 
G_M000_IG38:                ;; offset=0x07E9
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
       jge      SHORT G_M000_IG41
 
G_M000_IG39:                ;; offset=0x081E
       call     [System.Console:get_Error():System.IO.TextWriter]
       mov      r15, rax
       mov      r12d, dword ptr [rbp+0x39C]
       mov      rbx, gword ptr [rbp+0x448]
       cmp      r12d, dword ptr [rbx+0x08]
       jae      G_M000_IG138
       mov      esi, r12d
       mov      rsi, gword ptr [rbx+8*rsi+0x10]
       mov      rdi, ADDR
       call     [System.String:Concat(System.String,System.String):System.String]
       mov      rsi, rax
       mov      rdi, r15
       mov      rax, qword ptr [r15]
       mov      rax, qword ptr [rax+0x68]
       call     [rax+0x30]System.IO.TextWriter:WriteLine(System.String):this
       mov      eax, 1
 
G_M000_IG40:                ;; offset=0x086C
       add      rsp, 0x5A8
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG41:                ;; offset=0x087E
       mov      rdi, gword ptr [rbp+0x468]
       cmp      dword ptr [rdi], edi
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:ContextBytes():ubyte[]:this]
       test     rax, rax
       jne      SHORT G_M000_IG43
 
G_M000_IG42:                ;; offset=0x0892
       xor      rdi, rdi
       xor      ecx, ecx
       jmp      SHORT G_M000_IG44
 
G_M000_IG43:                ;; offset=0x0898
       lea      rdi, bword ptr [rax+0x10]
       mov      ecx, dword ptr [rax+0x08]
 
G_M000_IG44:                ;; offset=0x089F
       mov      rax, gword ptr [rbp+0x3D8]
       test     rax, rax
       jne      SHORT G_M000_IG46
 
G_M000_IG45:                ;; offset=0x08AB
       xor      rsi, rsi
       xor      r9d, r9d
       jmp      SHORT G_M000_IG47
 
G_M000_IG46:                ;; offset=0x08B2
       lea      rsi, bword ptr [rax+0x10]
       mov      gword ptr [rbp+0x3D8], rax
       mov      r9d, dword ptr [rax+0x08]
       mov      rax, gword ptr [rbp+0x3D8]
 
G_M000_IG47:                ;; offset=0x08C8
       cmp      ecx, r9d
       mov      gword ptr [rbp+0x3D8], rax
       jne      G_M000_IG95
 
G_M000_IG48:                ;; offset=0x08D8
       mov      edx, r9d
       call     [System.SpanHelpers:SequenceEqual(byref,byref,ulong):bool]
       test     eax, eax
       je       G_M000_IG95
 
G_M000_IG49:                ;; offset=0x08E9
       mov      eax, dword ptr [rbp+0x39C]
       inc      eax
       mov      rsi, gword ptr [rbp+0x450]
       cmp      dword ptr [rsi+0x08], eax
       mov      ecx, eax
       jg       G_M000_IG38
 
G_M000_IG50:                ;; offset=0x0903
       jmp      G_M000_IG37
 
G_M000_IG51:                ;; offset=0x0908
       xor      edx, edx
       xor      eax, eax
       mov      rdx, gword ptr [rbp+0x450]
       cmp      dword ptr [rdx+0x08], 0
       jg       G_M000_IG65
 
G_M000_IG52:                ;; offset=0x091D
       call     [Armonik.Ffi.Bdn.ProcCpu:Wall():long]
       mov      rcx, qword ptr [rbp+0x3C0]
       sub      rax, rcx
       vxorps   xmm0, xmm0, xmm0
       vcvtsi2sd xmm0, xmm0, rax
       vdivsd   xmm0, xmm0, qword ptr [reloc @RWD00]
       vmovsd   xmm1, qword ptr [reloc @RWD08]
       vucomisd xmm1, xmm0
       mov      qword ptr [rbp+0x3C0], rcx
       ja       SHORT G_M000_IG51
 
G_M000_IG53:                ;; offset=0x0953
       mov      rax, gword ptr [rbp+0x450]
       mov      gword ptr [rbp-0x100], rax
       mov      esi, dword ptr [rax+0x08]
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWARR_1_VC
       mov      gword ptr [rbp+0x3B8], rax
       mov      rsi, gword ptr [rbp-0x100]
       mov      esi, dword ptr [rsi+0x08]
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWARR_1_OBJ
       mov      rcx, gword ptr [rbp+0x3E8]
       lea      rdi, bword ptr [rcx+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       xor      eax, eax
       mov      dword ptr [rbp+0x394], eax
       mov      rdi, gword ptr [rbp+0x450]
       cmp      dword ptr [rdi+0x08], 0
       jg       G_M000_IG63
 
G_M000_IG54:                ;; offset=0x09BF
       xor      eax, eax
       xor      edx, edx
       test     ebx, ebx
       mov      gword ptr [rbp+0x438], r12
       mov      eax, edx
       mov      r12, gword ptr [rbp+0x3C8]
       jg       SHORT G_M000_IG56
 
G_M000_IG55:                ;; offset=0x09D7
       mov      ecx, dword ptr [r12+0x08]
       dec      ecx
       cmp      ecx, 1
       jge      G_M000_IG105
       jmp      G_M000_IG104
 
G_M000_IG56:                ;; offset=0x09EC
       xor      edi, edi
       mov      rsi, gword ptr [rbp+0x450]
       cmp      dword ptr [rsi+0x08], 0
       mov      dword ptr [rbp+0x390], eax
       mov      ecx, edi
       jg       SHORT G_M000_IG58
 
G_M000_IG57:                ;; offset=0x0A03
       mov      eax, dword ptr [rbp+0x390]
       inc      eax
       cmp      eax, ebx
       jge      SHORT G_M000_IG55
       jmp      SHORT G_M000_IG56
 
G_M000_IG58:                ;; offset=0x0A11
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
       mov      gword ptr [rbp+0x440], r13
       mov      r13, rsi
       mov      rax, gword ptr [rbp+0x3E8]
       mov      ecx, dword ptr [rbp+0x388]
       mov      r8d, dword ptr [rbp+0x414]
 
G_M000_IG59:                ;; offset=0x0A83
       cmp      ecx, dword ptr [r12+0x08]
       jae      G_M000_IG138
       mov      dword ptr [rbp+0x388], ecx
       mov      esi, ecx
       cmp      qword ptr [r12+8*rsi+0x10], r13
       mov      dword ptr [rbp+0x414], r8d
       jle      SHORT G_M000_IG62
 
G_M000_IG60:                ;; offset=0x0AA4
       mov      gword ptr [rbp+0x3E8], rax
       mov      rsi, gword ptr [rax+0x08]
       mov      rdi, gword ptr [rbp+0x468]
       xor      edx, edx
       cmp      dword ptr [rdi], edi
       call     [Armonik.Ffi.Harness.CoreFfi_UploadResultDataMessage:EncodeInto(Armonik.Ffi.Facade.UploadResultDataMessage,bool):int:this]
       inc      r13
       mov      eax, dword ptr [rbp+0x388]
       cmp      eax, dword ptr [r12+0x08]
       jae      G_M000_IG138
       mov      ecx, eax
       cmp      qword ptr [r12+8*rcx+0x10], r13
       jle      SHORT G_M000_IG61
       mov      dword ptr [rbp+0x388], eax
       mov      rax, gword ptr [rbp+0x3E8]
       jmp      SHORT G_M000_IG60
 
G_M000_IG61:                ;; offset=0x0AEC
       mov      dword ptr [rbp+0x388], eax
       jmp      G_M000_IG97
 
G_M000_IG62:                ;; offset=0x0AF7
       mov      gword ptr [rbp+0x3E8], rax
       jmp      G_M000_IG97
 
G_M000_IG63:                ;; offset=0x0B03
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      rcx, gword ptr [rbp+0x3E8]
       mov      rdi, gword ptr [rcx+0x10]
       mov      gword ptr [rbp-0x98], rdi
       mov      gword ptr [rbp-0x90], rax
       mov      rdi, rax
       call     [System.Collections.Generic.List`1[double]:.ctor():this]
       movsxd   rsi, dword ptr [rbp+0x394]
       mov      rdi, gword ptr [rbp-0x98]
       mov      rdx, gword ptr [rbp-0x90]
       call     CORINFO_HELP_ARRADDR_ST
       mov      edx, dword ptr [rbp+0x394]
       inc      edx
       mov      rdi, gword ptr [rbp+0x450]
       cmp      dword ptr [rdi+0x08], edx
       mov      dword ptr [rbp+0x394], edx
       jg       SHORT G_M000_IG63
 
G_M000_IG64:                ;; offset=0x0B68
       jmp      G_M000_IG54
 
G_M000_IG65:                ;; offset=0x0B6D
       lea      rdx, [rbp+0x450]
       mov      rdi, gword ptr [rbp+0x3E8]
       mov      dword ptr [rbp+0x398], eax
       mov      esi, eax
       call     [Armonik.Ffi.Bdn.StrSweep+<>c__DisplayClass2_1:<Run>g__Iters|1(int,byref):long:this]
       mov      ecx, dword ptr [rbp+0x398]
       mov      rdx, gword ptr [rbp+0x3C8]
       cmp      ecx, dword ptr [rdx+0x08]
       jae      G_M000_IG138
       mov      edi, ecx
       mov      gword ptr [rbp+0x3C8], rdx
       mov      qword ptr [rdx+8*rdi+0x10], rax
       inc      ecx
       mov      rax, gword ptr [rbp+0x450]
       cmp      dword ptr [rax+0x08], ecx
       mov      eax, ecx
       jg       SHORT G_M000_IG65
 
G_M000_IG66:                ;; offset=0x0BBD
       jmp      G_M000_IG52
 
G_M000_IG67:                ;; offset=0x0BC2
       mov      rdi, gword ptr [rbp+0x310]
       call     [System.Console:WriteLine(System.String)]
 
G_M000_IG68:                ;; offset=0x0BCF
       lea      rdi, [rbp+0x308]
       mov      rsi, ADDR
       call     [System.Collections.Generic.List`1+Enumerator[System.__Canon]:MoveNext():bool:this]
       test     eax, eax
       jne      SHORT G_M000_IG67
       xor      eax, eax
       test     r13d, r13d
       setne    al
 
G_M000_IG69:                ;; offset=0x0BF2
       add      rsp, 0x5A8
       pop      rbx
       pop      r12
       pop      r13
       pop      r14
       pop      r15
       pop      rbp
       ret      
 
G_M000_IG70:                ;; offset=0x0C04
       call     [System.Console:get_Error():System.IO.TextWriter]
       mov      gword ptr [rbp-0x88], rax
       mov      rdi, ADDR
       mov      esi, 6
       call     CORINFO_HELP_NEWARR_1_OBJ
       mov      rdi, ADDR
       mov      gword ptr [rax+0x10], rdi
       mov      gword ptr [rbp-0x80], rax
       lea      rdi, bword ptr [rax+0x18]
       mov      rsi, gword ptr [rbp+0x408]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      rax, gword ptr [rbp-0x80]
       mov      gword ptr [rax+0x20], rdi
       mov      edi, dword ptr [rbp+0x3F0]
       call     [System.Number:Int32ToDecStr(int):System.String]
       mov      rcx, gword ptr [rbp-0x80]
       lea      rdi, bword ptr [rcx+0x28]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      rax, gword ptr [rbp-0x80]
       mov      gword ptr [rax+0x30], rdi
       mov      ecx, dword ptr [rbp+0x39C]
       mov      rdx, gword ptr [rbp+0x448]
       cmp      ecx, dword ptr [rdx+0x08]
       jae      G_M000_IG138
       mov      dword ptr [rbp+0x39C], ecx
       mov      edi, ecx
       mov      gword ptr [rbp+0x448], rdx
       mov      rsi, gword ptr [rdx+8*rdi+0x10]
       mov      gword ptr [rbp-0x80], rax
       lea      rdi, bword ptr [rax+0x38]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, gword ptr [rbp-0x80]
       call     [System.String:Concat(System.String[]):System.String]
       mov      rsi, rax
       mov      rdi, gword ptr [rbp-0x88]
       mov      rax, qword ptr [rdi]
       mov      rax, qword ptr [rax+0x68]
       call     [rax+0x30]System.IO.TextWriter:WriteLine(System.String):this
       mov      r11d, dword ptr [rbp+0x430]
       inc      r11d
       mov      dword ptr [rbp+0x430], r11d
 
G_M000_IG71:                ;; offset=0x0CED
       jmp      G_M000_IG17
 
G_M000_IG72:                ;; offset=0x0CF2
       call     [Armonik.Ffi.Bdn.ProcCpu:Ns():long]
       mov      r13, rax
       call     [Armonik.Ffi.Bdn.ProcCpu:Wall():long]
       sub      r13, qword ptr [rbp+0x380]
       vxorps   xmm0, xmm0, xmm0
       vcvtsi2sd xmm0, xmm0, r13
       mov      r13, qword ptr [rbp-0x58]
       vxorps   xmm1, xmm1, xmm1
       vcvtsi2sd xmm1, xmm1, qword ptr [r12+8*r13+0x10]
       vdivsd   xmm0, xmm0, xmm1
       sub      rax, qword ptr [rbp+0x378]
       vxorps   xmm1, xmm1, xmm1
       vcvtsi2sd xmm1, xmm1, rax
       vxorps   xmm2, xmm2, xmm2
       vcvtsi2sd xmm2, xmm2, qword ptr [r12+8*r13+0x10]
       vdivsd   xmm1, xmm1, xmm2
       vmovsd   qword ptr [rbp+0x360], xmm1
       mov      rax, gword ptr [rbp+0x3E8]
       mov      rcx, gword ptr [rax+0x10]
       mov      edx, dword ptr [rbp+0x388]
       cmp      edx, dword ptr [rcx+0x08]
       jae      G_M000_IG138
       mov      rdi, gword ptr [rcx+8*r13+0x10]
       inc      dword ptr [rdi+0x14]
       mov      rsi, gword ptr [rdi+0x08]
       mov      r8d, dword ptr [rdi+0x10]
       cmp      dword ptr [rsi+0x08], r8d
       jbe      SHORT G_M000_IG73
       lea      ecx, [r8+0x01]
       mov      dword ptr [rdi+0x10], ecx
       mov      edi, r8d
       vmovsd   qword ptr [rsi+8*rdi+0x10], xmm0
       vmovsd   qword ptr [rbp+0x368], xmm0
       jmp      SHORT G_M000_IG74
 
G_M000_IG73:                ;; offset=0x0D95
       vmovsd   qword ptr [rbp+0x368], xmm0
       call     [System.Collections.Generic.List`1[double]:AddWithResize(double):this]
 
G_M000_IG74:                ;; offset=0x0DA3
       mov      rdi, ADDR
       mov      esi, 8
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
       mov      eax, dword ptr [rbp+0x388]
       mov      rcx, gword ptr [rbp+0x448]
       cmp      eax, dword ptr [rcx+0x08]
       jae      G_M000_IG138
       mov      gword ptr [rbp+0x448], rcx
       mov      rsi, gword ptr [rcx+8*r13+0x10]
       mov      rdx, gword ptr [rbp-0xE8]
       lea      rdi, bword ptr [rdx+0x28]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      ecx, dword ptr [rbp+0x390]
       lea      edi, [rcx+0x01]
       mov      dword ptr [rax+0x08], edi
       mov      rdx, gword ptr [rbp-0xE8]
       lea      rdi, bword ptr [rdx+0x30]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
 
G_M000_IG75:                ;; offset=0x0E8E
       mov      rdi, gword ptr [rdi]
       vmovsd   xmm0, qword ptr [rbp+0x368]
       vmovsd   qword ptr [rbp-0x38], xmm0
       call     [System.Globalization.NumberFormatInfo:<GetInstance>g__GetProviderNonNull|58_0(System.IFormatProvider):System.Globalization.NumberFormatInfo]
       vmovsd   xmm0, qword ptr [rbp-0x38]
       mov      rsi, rax
       mov      rdi, ADDR
       call     [System.Number:FormatDouble(double,System.String,System.Globalization.NumberFormatInfo):System.String]
       mov      rcx, gword ptr [rbp-0xE8]
       lea      rdi, bword ptr [rcx+0x38]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      rdi, gword ptr [rdi]
       vmovsd   xmm1, qword ptr [rbp+0x360]
       vmovsd   qword ptr [rbp-0x40], xmm1
       call     [System.Globalization.NumberFormatInfo:<GetInstance>g__GetProviderNonNull|58_0(System.IFormatProvider):System.Globalization.NumberFormatInfo]
       vmovsd   xmm0, qword ptr [rbp-0x40]
       mov      rsi, rax
       mov      rdi, ADDR
       call     [System.Number:FormatDouble(double,System.String,System.Globalization.NumberFormatInfo):System.String]
       mov      rcx, gword ptr [rbp-0xE8]
       lea      rdi, bword ptr [rcx+0x40]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      rdi, qword ptr [r12+8*r13+0x10]
       mov      qword ptr [rax+0x08], rdi
       mov      r13, gword ptr [rbp-0xE8]
       lea      rdi, bword ptr [r13+0x48]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      esi, 1
       mov      rdx, r13
       call     [System.String:JoinCore(System.ReadOnlySpan`1[ushort],System.Object[]):System.String]
       mov      r13, gword ptr [rbp+0x440]
       inc      dword ptr [r13+0x14]
       mov      rcx, gword ptr [r13+0x08]
       mov      edx, dword ptr [r13+0x10]
       cmp      dword ptr [rcx+0x08], edx
       jbe      SHORT G_M000_IG77
 
G_M000_IG76:                ;; offset=0x0F75
       lea      edi, [rdx+0x01]
       mov      dword ptr [r13+0x10], edi
       mov      edi, edx
       lea      rdi, bword ptr [rcx+8*rdi+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       jmp      SHORT G_M000_IG78
 
G_M000_IG77:                ;; offset=0x0F8D
       mov      rdi, r13
       mov      rsi, rax
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG78:                ;; offset=0x0F99
       mov      ecx, dword ptr [rbp+0x38C]
       inc      ecx
       mov      rax, gword ptr [rbp+0x450]
       cmp      dword ptr [rax+0x08], ecx
       jg       G_M000_IG27
       jmp      G_M000_IG26
 
G_M000_IG79:                ;; offset=0x0FB6
       mov      ecx, dword ptr [r12+0x08]
       dec      ecx
       cmp      ecx, 1
       jge      SHORT G_M000_IG80
       jmp      SHORT G_M000_IG81
 
G_M000_IG80:                ;; offset=0x0FC4
       mov      ecx, 1
 
G_M000_IG81:                ;; offset=0x0FC9
       cmp      ecx, dword ptr [r12+0x08]
       jae      G_M000_IG138
       mov      edi, ecx
       mov      r12, qword ptr [r12+8*rdi+0x10]
       cmp      r12, 0x3E8
       jle      SHORT G_M000_IG82
       jmp      SHORT G_M000_IG83
 
G_M000_IG82:                ;; offset=0x0FE6
       mov      r12d, 0x3E8
 
G_M000_IG83:                ;; offset=0x0FEC
       xor      eax, eax
       mov      dword ptr [rbp+0x354], eax
 
G_M000_IG84:                ;; offset=0x0FF4
       xor      edi, edi
       mov      qword ptr [rbp-0x48], rdi
       lea      rdi, [rbp-0x48]
       mov      rsi, gword ptr [rbp+0x3E0]
       mov      edx, 3
       call     [System.Runtime.InteropServices.GCHandle:.ctor(System.Object,int):this]
       mov      rdi, qword ptr [rbp-0x48]
       mov      qword ptr [rbp+0x348], rdi
       lea      rdi, [rbp+0x348]
       call     [System.Runtime.InteropServices.GCHandle:Free():this]
       mov      eax, dword ptr [rbp+0x354]
       inc      eax
       cmp      eax, 3
       mov      dword ptr [rbp+0x354], eax
       jl       SHORT G_M000_IG84
       call     [Armonik.Ffi.Bdn.ProcCpu:Ns():long]
       mov      qword ptr [rbp+0x3A8], rax
       xor      ecx, ecx
       jmp      SHORT G_M000_IG86
 
G_M000_IG85:                ;; offset=0x104C
       xor      edi, edi
       mov      qword ptr [rbp-0x50], rdi
       lea      rdi, [rbp-0x50]
       mov      rsi, gword ptr [rbp+0x3E0]
       mov      edx, 3
       call     [System.Runtime.InteropServices.GCHandle:.ctor(System.Object,int):this]
       mov      rdi, qword ptr [rbp-0x50]
       mov      qword ptr [rbp+0x338], rdi
       lea      rdi, [rbp+0x338]
       call     [System.Runtime.InteropServices.GCHandle:Free():this]
       mov      rcx, qword ptr [rbp+0x340]
       inc      rcx
 
G_M000_IG86:                ;; offset=0x108A
       mov      qword ptr [rbp+0x340], rcx
       cmp      rcx, r12
       jl       SHORT G_M000_IG85
       call     [Armonik.Ffi.Bdn.ProcCpu:Ns():long]
       sub      rax, qword ptr [rbp+0x3A8]
       vxorps   xmm0, xmm0, xmm0
       vcvtsi2sd xmm0, xmm0, rax
       vxorps   xmm1, xmm1, xmm1
       vcvtsi2sd xmm1, xmm1, r12
       vdivsd   xmm0, xmm0, xmm1
       vmovsd   qword ptr [rbp+0x3A0], xmm0
       xor      r12d, r12d
       jmp      G_M000_IG89
 
G_M000_IG87:                ;; offset=0x10C9
       mov      rax, gword ptr [rbp+0x3E8]
       mov      rcx, gword ptr [rax+0x10]
       cmp      r12d, dword ptr [rcx+0x08]
       jae      G_M000_IG138
       mov      edi, r12d
       mov      rdx, gword ptr [rcx+8*rdi+0x10]
       mov      gword ptr [rbp-0xA0], rdx
       mov      rdi, ADDR
       mov      esi, 79
       call     CORINFO_HELP_GETSHARED_NONGCSTATIC_BASE
       mov      rdi, ADDR
       mov      rax, gword ptr [rdi]
       mov      rcx, gword ptr [rbp-0xA0]
       mov      gword ptr [rbp-0xA8], rcx
       mov      rsi, rax
       test     rsi, rsi
       mov      gword ptr [rbp-0xB0], rsi
       jne      G_M000_IG88
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0xB8], rax
       mov      rdi, ADDR
       mov      esi, 79
       call     CORINFO_HELP_GETSHARED_NONGCSTATIC_BASE
       mov      rdi, ADDR
       mov      rsi, gword ptr [rdi]
       test     rsi, rsi
       je       G_M000_IG122
       mov      rax, gword ptr [rbp-0xB8]
       lea      rdi, bword ptr [rax+0x08]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      rsi, gword ptr [rbp-0xB8]
       mov      qword ptr [rsi+0x18], rdi
       mov      rdi, ADDR
       mov      esi, 79
       call     CORINFO_HELP_GETSHARED_NONGCSTATIC_BASE
       mov      rdi, ADDR
       mov      rsi, gword ptr [rbp-0xB8]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rsi, gword ptr [rbp-0xB8]
       mov      gword ptr [rbp-0xB0], rsi
 
G_M000_IG88:                ;; offset=0x11CC
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0xF0], rax
       mov      rdi, rax
       mov      rsi, gword ptr [rbp-0xA8]
       mov      rdx, gword ptr [rbp-0xB0]
       xor      rcx, rcx
       xor      r8d, r8d
       xor      r9, r9
       call     [System.Linq.OrderedEnumerable`2[double,double]:.ctor(System.Collections.Generic.IEnumerable`1[double],System.Func`2[double,double],System.Collections.Generic.IComparer`1[double],bool,System.Linq.OrderedEnumerable`1[double]):this]
       mov      rdi, gword ptr [rbp-0xF0]
       call     [System.Linq.Enumerable:ToList[double](System.Collections.Generic.IEnumerable`1[double]):System.Collections.Generic.List`1[double]]
       mov      ecx, dword ptr [rax+0x10]
       mov      edx, ecx
       shr      edx, 31
       add      ecx, edx
       sar      ecx, 1
       cmp      ecx, dword ptr [rax+0x10]
       jae      G_M000_IG121
       mov      rdx, gword ptr [rax+0x08]
       cmp      ecx, dword ptr [rdx+0x08]
       jae      G_M000_IG138
       mov      edi, ecx
       vmovsd   xmm0, qword ptr [rdx+8*rdi+0x10]
       mov      r8, gword ptr [rbp+0x3B8]
       cmp      r12d, dword ptr [r8+0x08]
       jae      G_M000_IG138
       mov      edi, r12d
       mov      gword ptr [rbp+0x3B8], r8
       vmovsd   qword ptr [r8+8*rdi+0x10], xmm0
       inc      r12d
 
G_M000_IG89:                ;; offset=0x125D
       mov      rdi, gword ptr [rbp+0x450]
       cmp      dword ptr [rdi+0x08], r12d
       jg       G_M000_IG87
       mov      rdi, ADDR
       mov      r12, gword ptr [rdi]
       mov      rdi, ADDR
       mov      esi, 6
       call     CORINFO_HELP_NEWARR_1_OBJ
       mov      gword ptr [rbp-0xC0], rax
       lea      rdi, bword ptr [rax+0x10]
       mov      rsi, gword ptr [rbp+0x408]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      ecx, dword ptr [rbp+0x3F0]
       mov      dword ptr [rax+0x08], ecx
       mov      rdx, gword ptr [rbp-0xC0]
       lea      rdi, bword ptr [rdx+0x18]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      ecx, dword ptr [rbp+0x3D4]
       mov      dword ptr [rax+0x08], ecx
       mov      rdx, gword ptr [rbp-0xC0]
       lea      rdi, bword ptr [rdx+0x20]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0xC8], rax
       mov      rcx, gword ptr [rbp+0x448]
       mov      esi, dword ptr [rcx+0x08]
       xor      edi, edi
       call     [System.Linq.Enumerable:Range(int,int):System.Collections.Generic.IEnumerable`1[int]]
       mov      gword ptr [rbp-0xD0], rax
       mov      rcx, gword ptr [rbp+0x3E8]
       test     rcx, rcx
       je       G_M000_IG122
       mov      rdx, gword ptr [rbp-0xC8]
       lea      rdi, bword ptr [rdx+0x08]
       mov      rsi, rcx
 
G_M000_IG90:                ;; offset=0x1349
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdx, ADDR
       mov      rax, gword ptr [rbp-0xC8]
       mov      qword ptr [rax+0x18], rdx
       mov      rdx, rax
       mov      rsi, gword ptr [rbp-0xD0]
       mov      rdi, ADDR
       call     [System.Linq.Enumerable:Select[int,System.__Canon](System.Collections.Generic.IEnumerable`1[int],System.Func`2[int,System.__Canon]):System.Collections.Generic.IEnumerable`1[System.__Canon]]
       mov      rsi, rax
       mov      rdi, ADDR
       call     [System.String:Join(System.String,System.Collections.Generic.IEnumerable`1[System.String]):System.String]
       mov      rcx, gword ptr [rbp-0xC0]
       lea      rdi, bword ptr [rcx+0x28]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       vmovsd   xmm0, qword ptr [rbp+0x3A0]
       vmovsd   qword ptr [rax+0x08], xmm0
       mov      rcx, gword ptr [rbp-0xC0]
       lea      rdi, bword ptr [rcx+0x30]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, gword ptr [rbp+0x3B8]
       call     [System.Linq.Enumerable:MinFloat[double](System.Collections.Generic.IEnumerable`1[double]):double]
       mov      rdi, gword ptr [rbp+0x3B8]
       call     [System.Array:IndexOf[double](double[],double):int]
       mov      rcx, gword ptr [rbp+0x448]
       cmp      eax, dword ptr [rcx+0x08]
       jae      G_M000_IG138
       mov      edi, eax
       mov      gword ptr [rbp+0x448], rcx
       mov      rsi, gword ptr [rcx+8*rdi+0x10]
       mov      rdx, gword ptr [rbp-0xC0]
       lea      rdi, bword ptr [rdx+0x38]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, r12
       mov      rdx, gword ptr [rbp-0xC0]
       mov      rsi, ADDR
       call     [System.String:Format(System.IFormatProvider,System.String,System.Object[]):System.String]
       mov      r12, gword ptr [rbp+0x438]
 
G_M000_IG91:                ;; offset=0x143B
       inc      dword ptr [r12+0x14]
       mov      rcx, gword ptr [r12+0x08]
       mov      edx, dword ptr [r12+0x10]
       cmp      dword ptr [rcx+0x08], edx
       jbe      SHORT G_M000_IG92
       lea      edi, [rdx+0x01]
       mov      dword ptr [r12+0x10], edi
       cmp      edx, dword ptr [rcx+0x08]
       jae      G_M000_IG138
       mov      edi, edx
       lea      rdi, bword ptr [rcx+8*rdi+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       jmp      SHORT G_M000_IG93
 
G_M000_IG92:                ;; offset=0x1471
       mov      rdi, r12
       mov      rsi, rax
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG93:                ;; offset=0x147D
       lea      rsi, [rbp+0x450]
       xor      edi, edi
       call     [Armonik.Ffi.Bdn.StrSweep:<Run>g__Set|2_0(int,byref)]
       xor      edi, edi
       mov      dword ptr [(reloc ADDR)], edi
 
G_M000_IG94:                ;; offset=0x1494
       mov      dword ptr [(reloc ADDR)], edi
       mov      byte  ptr [(reloc ADDR)], 0
       mov      esi, dword ptr [rbp+0x3FC]
       inc      esi
       mov      rax, gword ptr [rbp+0x400]
       cmp      dword ptr [rax+0x08], esi
       jg       G_M000_IG07
       jmp      G_M000_IG05
 
G_M000_IG95:                ;; offset=0x14BE
       call     [System.Console:get_Error():System.IO.TextWriter]
       mov      gword ptr [rbp-0x88], rax
       mov      rdi, ADDR
       mov      esi, 6
       call     CORINFO_HELP_NEWARR_1_OBJ
       mov      rdi, ADDR
       mov      gword ptr [rax+0x10], rdi
       mov      gword ptr [rbp-0x80], rax
       lea      rdi, bword ptr [rax+0x18]
       mov      rsi, gword ptr [rbp+0x408]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      rax, gword ptr [rbp-0x80]
       mov      gword ptr [rax+0x20], rdi
       mov      edi, dword ptr [rbp+0x3F0]
       call     [System.Number:Int32ToDecStr(int):System.String]
       mov      rcx, gword ptr [rbp-0x80]
       lea      rdi, bword ptr [rcx+0x28]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      rax, gword ptr [rbp-0x80]
       mov      gword ptr [rax+0x30], rdi
       mov      ecx, dword ptr [rbp+0x39C]
       mov      rdx, gword ptr [rbp+0x448]
       cmp      ecx, dword ptr [rdx+0x08]
       jae      G_M000_IG138
       mov      dword ptr [rbp+0x39C], ecx
       mov      edi, ecx
       mov      gword ptr [rbp+0x448], rdx
       mov      rsi, gword ptr [rdx+8*rdi+0x10]
       mov      gword ptr [rbp-0x80], rax
       lea      rdi, bword ptr [rax+0x38]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, gword ptr [rbp-0x80]
       call     [System.String:Concat(System.String[]):System.String]
       mov      rsi, rax
       mov      rdi, gword ptr [rbp-0x88]
       mov      rax, qword ptr [rdi]
       mov      rax, qword ptr [rax+0x68]
       call     [rax+0x30]System.IO.TextWriter:WriteLine(System.String):this
       mov      eax, dword ptr [rbp+0x430]
       inc      eax
       mov      dword ptr [rbp+0x430], eax
 
G_M000_IG96:                ;; offset=0x15A4
       jmp      G_M000_IG49
 
G_M000_IG97:                ;; offset=0x15A9
       call     [Armonik.Ffi.Bdn.ProcCpu:Ns():long]
       mov      qword ptr [rbp-0x30], rax
       call     [Armonik.Ffi.Bdn.ProcCpu:Wall():long]
       mov      rdi, qword ptr [rbp-0x30]
       sub      rdi, qword ptr [rbp+0x380]
       vxorps   xmm0, xmm0, xmm0
       vcvtsi2sd xmm0, xmm0, rdi
       mov      ecx, dword ptr [rbp+0x388]
       cmp      ecx, dword ptr [r12+0x08]
       jae      G_M000_IG138
       mov      edi, ecx
       vxorps   xmm1, xmm1, xmm1
       vcvtsi2sd xmm1, xmm1, qword ptr [r12+8*rdi+0x10]
       vdivsd   xmm0, xmm0, xmm1
       mov      rdi, rax
       sub      rdi, qword ptr [rbp+0x378]
       vxorps   xmm1, xmm1, xmm1
       vcvtsi2sd xmm1, xmm1, rdi
       cmp      ecx, dword ptr [r12+0x08]
       jae      G_M000_IG138
       mov      edi, ecx
       vxorps   xmm2, xmm2, xmm2
       vcvtsi2sd xmm2, xmm2, qword ptr [r12+8*rdi+0x10]
       vdivsd   xmm1, xmm1, xmm2
       vmovsd   qword ptr [rbp+0x360], xmm1
       mov      rax, gword ptr [rbp+0x3E8]
       mov      rdi, gword ptr [rax+0x10]
       cmp      ecx, dword ptr [rdi+0x08]
       jae      G_M000_IG138
       mov      dword ptr [rbp+0x388], ecx
       mov      edx, ecx
       mov      rdi, gword ptr [rdi+8*rdx+0x10]
       inc      dword ptr [rdi+0x14]
       mov      rsi, gword ptr [rdi+0x08]
       mov      r8d, dword ptr [rdi+0x10]
       cmp      dword ptr [rsi+0x08], r8d
       jbe      SHORT G_M000_IG98
       lea      edx, [r8+0x01]
       mov      dword ptr [rdi+0x10], edx
       cmp      r8d, dword ptr [rsi+0x08]
       jae      G_M000_IG138
       mov      edi, r8d
       vmovsd   qword ptr [rsi+8*rdi+0x10], xmm0
       vmovsd   qword ptr [rbp+0x368], xmm0
       jmp      SHORT G_M000_IG99
 
G_M000_IG98:                ;; offset=0x167C
       vmovsd   qword ptr [rbp+0x368], xmm0
       call     [System.Collections.Generic.List`1[double]:AddWithResize(double):this]
 
G_M000_IG99:                ;; offset=0x168A
       mov      rdi, ADDR
       mov      esi, 8
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
       mov      eax, dword ptr [rbp+0x388]
       mov      rcx, gword ptr [rbp+0x448]
       cmp      eax, dword ptr [rcx+0x08]
       jae      G_M000_IG138
       mov      dword ptr [rbp+0x388], eax
       mov      edi, eax
       mov      gword ptr [rbp+0x448], rcx
       mov      rsi, gword ptr [rcx+8*rdi+0x10]
       mov      rdx, gword ptr [rbp-0xE8]
       lea      rdi, bword ptr [rdx+0x28]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      ecx, dword ptr [rbp+0x390]
       lea      edi, [rcx+0x01]
       mov      dword ptr [rax+0x08], edi
       mov      rdx, gword ptr [rbp-0xE8]
       lea      rdi, bword ptr [rdx+0x30]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
 
G_M000_IG100:                ;; offset=0x1773
       mov      rdi, ADDR
       mov      rdi, gword ptr [rdi]
       vmovsd   xmm0, qword ptr [rbp+0x368]
       vmovsd   qword ptr [rbp-0x38], xmm0
       call     [System.Globalization.NumberFormatInfo:<GetInstance>g__GetProviderNonNull|58_0(System.IFormatProvider):System.Globalization.NumberFormatInfo]
       vmovsd   xmm0, qword ptr [rbp-0x38]
       mov      rsi, rax
       mov      rdi, ADDR
       call     [System.Number:FormatDouble(double,System.String,System.Globalization.NumberFormatInfo):System.String]
       mov      rcx, gword ptr [rbp-0xE8]
       lea      rdi, bword ptr [rcx+0x38]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      rdi, gword ptr [rdi]
       vmovsd   xmm1, qword ptr [rbp+0x360]
       vmovsd   qword ptr [rbp-0x40], xmm1
       call     [System.Globalization.NumberFormatInfo:<GetInstance>g__GetProviderNonNull|58_0(System.IFormatProvider):System.Globalization.NumberFormatInfo]
       vmovsd   xmm0, qword ptr [rbp-0x40]
       mov      rsi, rax
       mov      rdi, ADDR
       call     [System.Number:FormatDouble(double,System.String,System.Globalization.NumberFormatInfo):System.String]
       mov      rcx, gword ptr [rbp-0xE8]
       lea      rdi, bword ptr [rcx+0x40]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      ecx, dword ptr [rbp+0x388]
       cmp      ecx, dword ptr [r12+0x08]
       jae      G_M000_IG138
       mov      edi, ecx
       mov      rdi, qword ptr [r12+8*rdi+0x10]
       mov      qword ptr [rax+0x08], rdi
       mov      rcx, gword ptr [rbp-0xE8]
       lea      rdi, bword ptr [rcx+0x48]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      esi, 1
       mov      rdx, gword ptr [rbp-0xE8]
       call     [System.String:JoinCore(System.ReadOnlySpan`1[ushort],System.Object[]):System.String]
       mov      r13, gword ptr [rbp+0x440]
 
G_M000_IG101:                ;; offset=0x186A
       inc      dword ptr [r13+0x14]
       mov      rcx, gword ptr [r13+0x08]
       mov      edx, dword ptr [r13+0x10]
       cmp      dword ptr [rcx+0x08], edx
       jbe      SHORT G_M000_IG102
       lea      edi, [rdx+0x01]
       mov      dword ptr [r13+0x10], edi
       cmp      edx, dword ptr [rcx+0x08]
       jae      G_M000_IG138
       mov      edi, edx
       lea      rdi, bword ptr [rcx+8*rdi+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       jmp      SHORT G_M000_IG103
 
G_M000_IG102:                ;; offset=0x189C
       mov      rdi, r13
       mov      rsi, rax
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG103:                ;; offset=0x18A8
       mov      ecx, dword ptr [rbp+0x38C]
       inc      ecx
       mov      rax, gword ptr [rbp+0x450]
       cmp      dword ptr [rax+0x08], ecx
       jg       G_M000_IG58
       jmp      G_M000_IG57
 
G_M000_IG104:                ;; offset=0x18C5
       jmp      SHORT G_M000_IG106
 
G_M000_IG105:                ;; offset=0x18C7
       mov      ecx, 1
 
G_M000_IG106:                ;; offset=0x18CC
       cmp      ecx, dword ptr [r12+0x08]
       jae      G_M000_IG138
       mov      edi, ecx
       mov      r12, qword ptr [r12+8*rdi+0x10]
       cmp      r12, 0x3E8
       jle      SHORT G_M000_IG107
       jmp      SHORT G_M000_IG108
 
G_M000_IG107:                ;; offset=0x18E9
       mov      r12d, 0x3E8
 
G_M000_IG108:                ;; offset=0x18EF
       xor      eax, eax
       mov      dword ptr [rbp+0x354], eax
 
G_M000_IG109:                ;; offset=0x18F7
       xor      edi, edi
       mov      qword ptr [rbp-0x48], rdi
       lea      rdi, [rbp-0x48]
       mov      rsi, gword ptr [rbp+0x3E0]
       mov      edx, 3
       call     [System.Runtime.InteropServices.GCHandle:.ctor(System.Object,int):this]
       mov      rdi, qword ptr [rbp-0x48]
       mov      qword ptr [rbp+0x348], rdi
       lea      rdi, [rbp+0x348]
       call     [System.Runtime.InteropServices.GCHandle:Free():this]
       mov      eax, dword ptr [rbp+0x354]
       inc      eax
       cmp      eax, 3
       mov      dword ptr [rbp+0x354], eax
       jl       SHORT G_M000_IG109
       call     [Armonik.Ffi.Bdn.ProcCpu:Ns():long]
       mov      qword ptr [rbp+0x3A8], rax
       xor      ecx, ecx
       jmp      SHORT G_M000_IG111
 
G_M000_IG110:                ;; offset=0x194F
       xor      edi, edi
       mov      qword ptr [rbp-0x50], rdi
       lea      rdi, [rbp-0x50]
       mov      rsi, gword ptr [rbp+0x3E0]
       mov      edx, 3
       call     [System.Runtime.InteropServices.GCHandle:.ctor(System.Object,int):this]
       mov      rdi, qword ptr [rbp-0x50]
       mov      qword ptr [rbp+0x338], rdi
       lea      rdi, [rbp+0x338]
       call     [System.Runtime.InteropServices.GCHandle:Free():this]
       mov      rcx, qword ptr [rbp+0x340]
       inc      rcx
 
G_M000_IG111:                ;; offset=0x198D
       mov      qword ptr [rbp+0x340], rcx
       cmp      rcx, r12
       jl       SHORT G_M000_IG110
       call     [Armonik.Ffi.Bdn.ProcCpu:Ns():long]
       sub      rax, qword ptr [rbp+0x3A8]
       vxorps   xmm0, xmm0, xmm0
       vcvtsi2sd xmm0, xmm0, rax
       vxorps   xmm1, xmm1, xmm1
       vcvtsi2sd xmm1, xmm1, r12
       vdivsd   xmm0, xmm0, xmm1
       vmovsd   qword ptr [rbp+0x3A0], xmm0
       xor      r12d, r12d
       jmp      G_M000_IG114
 
G_M000_IG112:                ;; offset=0x19CC
       mov      rax, gword ptr [rbp+0x3E8]
       mov      rcx, gword ptr [rax+0x10]
       cmp      r12d, dword ptr [rcx+0x08]
       jae      G_M000_IG138
       mov      edi, r12d
       mov      rcx, gword ptr [rcx+8*rdi+0x10]
       mov      gword ptr [rbp-0xA0], rcx
       mov      rdi, ADDR
       mov      esi, 79
       call     CORINFO_HELP_GETSHARED_NONGCSTATIC_BASE
       mov      rdi, ADDR
       mov      rax, gword ptr [rdi]
       mov      rdx, rax
       mov      rsi, gword ptr [rbp-0xA0]
       mov      gword ptr [rbp-0xA8], rsi
       mov      rcx, rdx
       test     rcx, rcx
       mov      gword ptr [rbp-0xB0], rcx
       jne      G_M000_IG113
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0xB8], rax
       mov      rdi, ADDR
       mov      esi, 79
       call     CORINFO_HELP_GETSHARED_NONGCSTATIC_BASE
       mov      rdi, ADDR
       mov      rsi, gword ptr [rdi]
       test     rsi, rsi
       je       G_M000_IG122
       mov      rcx, gword ptr [rbp-0xB8]
       lea      rdi, bword ptr [rcx+0x08]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       mov      rcx, gword ptr [rbp-0xB8]
       mov      qword ptr [rcx+0x18], rdi
       mov      rdi, ADDR
       mov      esi, 79
       call     CORINFO_HELP_GETSHARED_NONGCSTATIC_BASE
       mov      rdi, ADDR
       mov      rsi, gword ptr [rbp-0xB8]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rcx, gword ptr [rbp-0xB8]
       mov      gword ptr [rbp-0xB0], rcx
 
G_M000_IG113:                ;; offset=0x1AD2
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0xF0], rax
       mov      rdi, rax
       mov      rsi, gword ptr [rbp-0xA8]
       mov      rdx, gword ptr [rbp-0xB0]
       xor      rcx, rcx
       xor      r8d, r8d
       xor      r9, r9
       call     [System.Linq.OrderedEnumerable`2[double,double]:.ctor(System.Collections.Generic.IEnumerable`1[double],System.Func`2[double,double],System.Collections.Generic.IComparer`1[double],bool,System.Linq.OrderedEnumerable`1[double]):this]
       mov      rdi, gword ptr [rbp-0xF0]
       call     [System.Linq.Enumerable:ToList[double](System.Collections.Generic.IEnumerable`1[double]):System.Collections.Generic.List`1[double]]
       mov      ecx, dword ptr [rax+0x10]
       mov      edi, ecx
       shr      edi, 31
       add      ecx, edi
       sar      ecx, 1
       cmp      ecx, dword ptr [rax+0x10]
       jae      G_M000_IG121
       mov      rdx, gword ptr [rax+0x08]
       cmp      ecx, dword ptr [rdx+0x08]
       jae      G_M000_IG138
       mov      edi, ecx
       vmovsd   xmm0, qword ptr [rdx+8*rdi+0x10]
       mov      rax, gword ptr [rbp+0x3B8]
       cmp      r12d, dword ptr [rax+0x08]
       jae      G_M000_IG138
       mov      edi, r12d
       mov      gword ptr [rbp+0x3B8], rax
       vmovsd   qword ptr [rax+8*rdi+0x10], xmm0
       inc      r12d
 
G_M000_IG114:                ;; offset=0x1B62
       mov      rdi, gword ptr [rbp+0x450]
       cmp      dword ptr [rdi+0x08], r12d
       jg       G_M000_IG112
       mov      rdi, ADDR
       mov      r12, gword ptr [rdi]
       mov      rdi, ADDR
       mov      esi, 6
       call     CORINFO_HELP_NEWARR_1_OBJ
       mov      gword ptr [rbp-0xC0], rax
       lea      rdi, bword ptr [rax+0x10]
       mov      rsi, gword ptr [rbp+0x408]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      ecx, dword ptr [rbp+0x3F0]
       mov      dword ptr [rax+0x08], ecx
       mov      rcx, gword ptr [rbp-0xC0]
       lea      rdi, bword ptr [rcx+0x18]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      ecx, dword ptr [rbp+0x3D4]
       mov      dword ptr [rax+0x08], ecx
       mov      rcx, gword ptr [rbp-0xC0]
       lea      rdi, bword ptr [rcx+0x20]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0xC8], rax
       mov      rcx, gword ptr [rbp+0x448]
       mov      esi, dword ptr [rcx+0x08]
       xor      edi, edi
       call     [System.Linq.Enumerable:Range(int,int):System.Collections.Generic.IEnumerable`1[int]]
       mov      rcx, gword ptr [rbp+0x3E8]
       test     rcx, rcx
       mov      gword ptr [rbp-0xD0], rax
       je       G_M000_IG122
       mov      r8, gword ptr [rbp-0xC8]
       mov      gword ptr [rbp-0xC8], r8
       lea      rdi, bword ptr [r8+0x08]
       mov      rsi, rcx
 
G_M000_IG115:                ;; offset=0x1C55
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdx, ADDR
       mov      rsi, gword ptr [rbp-0xC8]
       mov      qword ptr [rsi+0x18], rdx
       mov      rdx, rsi
       mov      rsi, gword ptr [rbp-0xD0]
       mov      rdi, ADDR
       call     [System.Linq.Enumerable:Select[int,System.__Canon](System.Collections.Generic.IEnumerable`1[int],System.Func`2[int,System.__Canon]):System.Collections.Generic.IEnumerable`1[System.__Canon]]
       mov      rsi, rax
       mov      rdi, ADDR
       call     [System.String:Join(System.String,System.Collections.Generic.IEnumerable`1[System.String]):System.String]
       mov      rcx, gword ptr [rbp-0xC0]
       lea      rdi, bword ptr [rcx+0x28]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       vmovsd   xmm0, qword ptr [rbp+0x3A0]
       vmovsd   qword ptr [rax+0x08], xmm0
       mov      rcx, gword ptr [rbp-0xC0]
       lea      rdi, bword ptr [rcx+0x30]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, gword ptr [rbp+0x3B8]
       call     [System.Linq.Enumerable:MinFloat[double](System.Collections.Generic.IEnumerable`1[double]):double]
       mov      rdi, gword ptr [rbp+0x3B8]
       call     [System.Array:IndexOf[double](double[],double):int]
       mov      rcx, gword ptr [rbp+0x448]
       cmp      eax, dword ptr [rcx+0x08]
       jae      G_M000_IG138
       mov      edi, eax
       mov      gword ptr [rbp+0x448], rcx
       mov      rsi, gword ptr [rcx+8*rdi+0x10]
       mov      rax, gword ptr [rbp-0xC0]
       lea      rdi, bword ptr [rax+0x38]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdi, r12
       mov      rdx, gword ptr [rbp-0xC0]
       mov      rsi, ADDR
       call     [System.String:Format(System.IFormatProvider,System.String,System.Object[]):System.String]
       mov      r12, gword ptr [rbp+0x438]
 
G_M000_IG116:                ;; offset=0x1D47
       inc      dword ptr [r12+0x14]
       mov      rcx, gword ptr [r12+0x08]
       mov      edx, dword ptr [r12+0x10]
       cmp      dword ptr [rcx+0x08], edx
       jbe      SHORT G_M000_IG117
       lea      edi, [rdx+0x01]
       mov      dword ptr [r12+0x10], edi
       cmp      edx, dword ptr [rcx+0x08]
       jae      G_M000_IG138
       mov      edi, edx
       lea      rdi, bword ptr [rcx+8*rdi+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       jmp      SHORT G_M000_IG118
 
G_M000_IG117:                ;; offset=0x1D7D
       mov      rdi, r12
       mov      rsi, rax
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG118:                ;; offset=0x1D89
       lea      rsi, [rbp+0x450]
       xor      edi, edi
       call     [Armonik.Ffi.Bdn.StrSweep:<Run>g__Set|2_0(int,byref)]
       xor      edi, edi
       mov      dword ptr [(reloc ADDR)], edi
 
G_M000_IG119:                ;; offset=0x1DA0
       mov      dword ptr [(reloc ADDR)], edi
       mov      byte  ptr [(reloc ADDR)], 0
       mov      eax, dword ptr [rbp+0x3FC]
       inc      eax
       mov      rcx, gword ptr [rbp+0x400]
       cmp      dword ptr [rcx+0x08], eax
       jg       G_M000_IG35
       jmp      G_M000_IG33
 
G_M000_IG120:                ;; offset=0x1DCA
       call     [System.IO.File:get_UTF8NoBOM():System.Text.Encoding]
       mov      rdx, rax
       mov      rsi, r13
       mov      rdi, r14
       call     [System.IO.File:WriteAllLines(System.String,System.Collections.Generic.IEnumerable`1[System.String],System.Text.Encoding)]
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      r15, rax
       mov      rdi, r15
       call     [System.Collections.Generic.List`1[System.__Canon]:.ctor():this]
       mov      edi, ebx
       call     [System.Number:Int32ToDecStr(int):System.String]
       mov      rsi, rax
       mov      rdi, ADDR
       mov      rdx, ADDR
       call     [System.String:Concat(System.String,System.String,System.String):System.String]
       inc      dword ptr [r15+0x14]
       mov      rdi, gword ptr [r15+0x08]
       mov      esi, dword ptr [r15+0x10]
       cmp      dword ptr [rdi+0x08], esi
       jbe      SHORT G_M000_IG123
       lea      ecx, [rsi+0x01]
       mov      dword ptr [r15+0x10], ecx
       cmp      esi, dword ptr [rdi+0x08]
       jae      G_M000_IG138
       lea      rdi, bword ptr [rdi+8*rsi+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       jmp      SHORT G_M000_IG124
 
G_M000_IG121:                ;; offset=0x1E4F
       call     [System.ThrowHelper:ThrowArgumentOutOfRange_IndexMustBeLessException()]
       int3     
 
G_M000_IG122:                ;; offset=0x1E56
       call     [System.MulticastDelegate:ThrowNullThisInDelegateToInstance()]
       int3     
 
G_M000_IG123:                ;; offset=0x1E5D
       mov      rdi, r15
       mov      rsi, rax
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG124:                ;; offset=0x1E69
       inc      dword ptr [r15+0x14]
       mov      rdi, gword ptr [r15+0x08]
       mov      esi, dword ptr [r15+0x10]
       cmp      dword ptr [rdi+0x08], esi
       jbe      SHORT G_M000_IG125
       lea      eax, [rsi+0x01]
       mov      dword ptr [r15+0x10], eax
       cmp      esi, dword ptr [rdi+0x08]
       jae      G_M000_IG138
       mov      rax, ADDR
       mov      gword ptr [rdi+8*rsi+0x10], rax
       jmp      SHORT G_M000_IG126
 
G_M000_IG125:                ;; offset=0x1E9B
       mov      rdi, r15
       mov      rsi, ADDR
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG126:                ;; offset=0x1EAE
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
       mov      r13d, dword ptr [rbp+0x430]
       test     r13d, r13d
       je       SHORT G_M000_IG127
       mov      gword ptr [rbp-0xD8], rbx
       mov      edi, r13d
       call     [System.Number:Int32ToDecStr(int):System.String]
       mov      rdi, rax
       mov      rsi, ADDR
       call     [System.String:Concat(System.String,System.String):System.String]
       jmp      SHORT G_M000_IG128
 
G_M000_IG127:                ;; offset=0x1F24
       mov      rdi, rbx
       mov      rax, ADDR
       mov      gword ptr [rbp-0xD8], rdi
 
G_M000_IG128:                ;; offset=0x1F38
       mov      esi, 3
       mov      rdi, gword ptr [rbp-0xD8]
       mov      rdx, rax
       call     CORINFO_HELP_ARRADDR_ST
       mov      rdi, rbx
       mov      esi, 4
       mov      rdx, ADDR
       call     CORINFO_HELP_ARRADDR_ST
       mov      rdi, rbx
       call     [System.String:Concat(System.String[]):System.String]
       inc      dword ptr [r15+0x14]
       mov      rdi, gword ptr [r15+0x08]
       mov      esi, dword ptr [r15+0x10]
       cmp      dword ptr [rdi+0x08], esi
       jbe      SHORT G_M000_IG129
       lea      ecx, [rsi+0x01]
       mov      dword ptr [r15+0x10], ecx
       cmp      esi, dword ptr [rdi+0x08]
       jae      G_M000_IG138
       lea      rdi, bword ptr [rdi+8*rsi+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       jmp      SHORT G_M000_IG130
 
G_M000_IG129:                ;; offset=0x1F9C
       mov      rdi, r15
       mov      rsi, rax
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG130:                ;; offset=0x1FA8
       inc      dword ptr [r15+0x14]
       mov      rdi, gword ptr [r15+0x08]
       mov      esi, dword ptr [r15+0x10]
       cmp      dword ptr [rdi+0x08], esi
       jbe      SHORT G_M000_IG131
       lea      eax, [rsi+0x01]
       mov      dword ptr [r15+0x10], eax
       cmp      esi, dword ptr [rdi+0x08]
       jae      G_M000_IG138
       mov      rax, ADDR
       mov      gword ptr [rdi+8*rsi+0x10], rax
       jmp      SHORT G_M000_IG132
 
G_M000_IG131:                ;; offset=0x1FDA
       mov      rdi, r15
       mov      rsi, ADDR
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG132:                ;; offset=0x1FED
       mov      rsi, gword ptr [rbp+0x448]
       mov      rdi, ADDR
       call     [System.String:Join(System.String,System.String[]):System.String]
       mov      rsi, rax
       mov      rdi, ADDR
       mov      rdx, ADDR
       call     [System.String:Concat(System.String,System.String,System.String):System.String]
       inc      dword ptr [r15+0x14]
       mov      rdi, gword ptr [r15+0x08]
       mov      esi, dword ptr [r15+0x10]
       cmp      dword ptr [rdi+0x08], esi
       jbe      SHORT G_M000_IG133
       lea      ecx, [rsi+0x01]
       mov      dword ptr [r15+0x10], ecx
       cmp      esi, dword ptr [rdi+0x08]
       jae      G_M000_IG138
       lea      rdi, bword ptr [rdi+8*rsi+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       jmp      SHORT G_M000_IG134
 
G_M000_IG133:                ;; offset=0x2051
       mov      rdi, r15
       mov      rsi, rax
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG134:                ;; offset=0x205D
       mov      rdi, ADDR
       mov      esi, 79
       call     CORINFO_HELP_GETSHARED_NONGCSTATIC_BASE
       mov      rdi, ADDR
       mov      rdx, gword ptr [rdi]
       mov      rbx, gword ptr [rbp+0x448]
       test     rdx, rdx
       jne      G_M000_IG135
       mov      rdi, ADDR
       call     CORINFO_HELP_NEWSFAST
       mov      gword ptr [rbp-0xE0], rax
       mov      rdi, ADDR
       mov      esi, 79
       call     CORINFO_HELP_GETSHARED_NONGCSTATIC_BASE
       mov      rsi, ADDR
       mov      rsi, gword ptr [rsi]
       mov      rdi, gword ptr [rbp-0xE0]
       mov      rdx, ADDR
       call     [System.MulticastDelegate:CtorClosed(System.Object,long):this]
       mov      rdi, ADDR
       mov      esi, 79
       call     CORINFO_HELP_GETSHARED_NONGCSTATIC_BASE
       mov      rdi, ADDR
       mov      rsi, gword ptr [rbp-0xE0]
       call     CORINFO_HELP_ASSIGN_REF
       mov      rdx, gword ptr [rbp-0xE0]
 
G_M000_IG135:                ;; offset=0x210D
       mov      rsi, rbx
       mov      rdi, ADDR
       call     [System.Linq.Enumerable:Select[System.__Canon,System.__Canon](System.Collections.Generic.IEnumerable`1[System.__Canon],System.Func`2[System.__Canon,System.__Canon]):System.Collections.Generic.IEnumerable`1[System.__Canon]]
       mov      rdi, rax
       call     [System.String:Concat(System.Collections.Generic.IEnumerable`1[System.String]):System.String]
       mov      rsi, rax
       mov      rdi, ADDR
       mov      rdx, ADDR
       call     [System.String:Concat(System.String,System.String,System.String):System.String]
       inc      dword ptr [r15+0x14]
       mov      rdi, gword ptr [r15+0x08]
       mov      esi, dword ptr [r15+0x10]
       cmp      dword ptr [rdi+0x08], esi
       jbe      SHORT G_M000_IG136
       lea      ecx, [rsi+0x01]
       mov      dword ptr [r15+0x10], ecx
       cmp      esi, dword ptr [rdi+0x08]
       jae      SHORT G_M000_IG138
       lea      rdi, bword ptr [rdi+8*rsi+0x10]
       mov      rsi, rax
       call     CORINFO_HELP_ASSIGN_REF
       jmp      SHORT G_M000_IG137
 
G_M000_IG136:                ;; offset=0x2172
       mov      rdi, r15
       mov      rsi, rax
       call     [System.Collections.Generic.List`1[System.__Canon]:AddWithResize(System.__Canon):this]
 
G_M000_IG137:                ;; offset=0x217E
       mov      rdi, r15
       mov      rsi, r12
       call     [System.Collections.Generic.List`1[System.__Canon]:AddRange(System.Collections.Generic.IEnumerable`1[System.__Canon]):this]
       mov      rdi, r14
       mov      rsi, ADDR
       call     [System.IO.Path:ChangeExtension(System.String,System.String):System.String]
       mov      r12, rax
       call     [System.IO.File:get_UTF8NoBOM():System.Text.Encoding]
       mov      rdx, rax
       mov      rdi, r12
       mov      rsi, r15
       call     [System.IO.File:WriteAllLines(System.String,System.Collections.Generic.IEnumerable`1[System.String],System.Text.Encoding)]
       lea      rsi, [rbp+0x308]
       mov      rdi, r15
       call     [System.Collections.Generic.List`1[System.__Canon]:GetEnumerator():System.Collections.Generic.List`1+Enumerator[System.__Canon]:this]
       jmp      G_M000_IG68
 
G_M000_IG138:                ;; offset=0x21CA
       call     CORINFO_HELP_RNGCHKFAIL
       int3     
 
RWD00  	dq	412E848000000000h	;      1000000
RWD08  	dq	408F400000000000h	;         1000

; Total bytes of code 8656

