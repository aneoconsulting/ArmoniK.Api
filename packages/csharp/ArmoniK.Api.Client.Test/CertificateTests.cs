// This file is part of the ArmoniK project
//
// Copyright (C) ANEO, 2021-2022. All rights reserved.
//   W. Kirschenmann   <wkirschenmann@aneo.fr>
//   J. Gurhem         <jgurhem@aneo.fr>
//   D. Dubuc          <ddubuc@aneo.fr>
//   L. Ziane Khodja   <lzianekhodja@aneo.fr>
//   F. Lemaitre       <flemaitre@aneo.fr>
//   S. Djebbar        <sdjebbar@aneo.fr>
//   J. Fonseca        <jfonseca@aneo.fr>
//
// Licensed under the Apache License, Version 2.0 (the "License")
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//         http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

using System.IO;
using System.Security.Cryptography.X509Certificates;

using ArmoniK.Api.Client.Options;
using ArmoniK.Api.Client.Submitter;

using NUnit.Framework;

namespace ArmoniK.Api.Client.Tests;

[TestFixture]
public class CertificateTests
{
  [Test]
  [TestCase(null,
            "certificate-rsa.pem",
            "privatersa.pem")]
  [TestCase(null,
            "certificate-rsa.pem",
            "privatersa.p8")]
  [TestCase("certificate-rsa.p12",
            "",
            "")]
  public void TestRSACertificate(string? p12Path,
                                 string  certPath,
                                 string  keyPath)
  {
    var basePath = Path.Combine(TestContext.CurrentContext.TestDirectory,
                                "TestFiles");
    var options = new GrpcClient
                  {
                    CertP12 = p12Path is null
                                ? ""
                                : Path.Combine(basePath,
                                               p12Path),
                    CertPem = Path.Combine(basePath,
                                           certPath),
                    KeyPem = Path.Combine(basePath,
                                          keyPath),
                  };
    var certificate = GrpcChannelFactory.GetCertificate(options);

    Assert.NotNull(certificate);
    Assert.NotNull(certificate.GetRSAPublicKey());
  }

  [Test]
  [TestCase(null,
            "certificate-ec.pem",
            "privateec.pem")]
  [TestCase("certificate-ec.p12",
            "",
            "")]
  public void TestECDSACertificate(string? p12Path,
                                   string  certPath,
                                   string  keyPath)
  {
    var basePath = Path.Combine(TestContext.CurrentContext.TestDirectory,
                                "TestFiles");
    var options = new GrpcClient
                  {
                    CertP12 = p12Path is null
                                ? ""
                                : Path.Combine(basePath,
                                               p12Path),
                    CertPem = Path.Combine(basePath,
                                           certPath),
                    KeyPem = Path.Combine(basePath,
                                          keyPath),
                  };

    var certificate = GrpcChannelFactory.GetCertificate(options);

    Assert.NotNull(certificate);
    Assert.NotNull(certificate.GetECDsaPublicKey());
  }
}
