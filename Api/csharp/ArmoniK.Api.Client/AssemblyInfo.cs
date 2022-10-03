using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

// In SDK-style projects such as this one, several assembly attributes that were historically
// defined in this file are now automatically added during build and populated with
// values defined in project properties. For details of which attributes are included
// and how to customise this process see: https://aka.ms/assembly-info-properties


// Setting ComVisible to false makes the types in this assembly not visible to COM
// components.  If you need to access a type in this assembly from COM, set the ComVisible
// attribute to true on that type.

[assembly: ComVisible(false)]

// The following GUID is for the ID of the typelib if this project is exposed to COM.

[assembly: Guid("b4b7b325-c430-4fcf-b743-8a3ce4f6b5a1")]

// to extract public key : sn.exe -tp kp.snk kp.pub
// to print the public key : sn.exe -p kp.pub
[assembly:
  InternalsVisibleTo("ArmoniK.Api.Client.Tests, PublicKey=0024000004800000940000000602000000240000525341310004000001000100b9cbe494cb23f1c9a351b8d0f211ba3f27afd44f1e683f1c08077b08372ad2649a9427e888c2aad68f010776c168f7a755e6ec591e48fcdd6928d2d6f1aeba06f7c3857437a5a15c7407756e17c3e1877a92eb5f9c82369731520f257bbca1f61a4caaa8aafc7aa40c5810cb81f16c68b4d4f8aa3044b09f7b417ca553bd53be")]
