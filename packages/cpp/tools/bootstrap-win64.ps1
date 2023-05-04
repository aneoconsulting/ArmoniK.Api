# Set directory for installation - Chocolatey does not lock
# down the directory if not the default
$InstallDir="$pwd\win64"
$env:VS100COMNTOOLS="C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools"

# Check if the installation directory for nasm exists
if (!(Test-Path $InstallDir -PathType Container)) {
    # Create the installation directory for nasm if it does not exist
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$env:ChocolateyInstall="$InstallDir"
$env:Path += ";$InstallDir\bin"
$env:Path += ";$InstallDir"

# If your PowerShell Execution policy is restrictive, you may
# not be able to get around that. Try setting your session to
# Bypass.
Set-ExecutionPolicy Bypass -Scope Process -Force;

Invoke-Item "RefreshEnv.cmd"

# All install options - offline, proxy, etc at
# https://chocolatey.org/install
iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))

$env:Path += ";$InstallDir\bin"

# Install CMake
choco install -y cmake.portable --force --installargs "ALLUSERS=0 INSTALLDIR=$InstallDir MSIINSTALLPERUSER=1 "

# Download and extract nasm
$Url = "https://www.nasm.us/pub/nasm/releasebuilds/2.16.01/win64/nasm-2.16.01-win64.zip"
$ZipFile = "$InstallDir\nasm.zip"

# Check if the zip file exists
if (!(Test-Path $ZipFile -PathType Leaf)) {
    # Download the nasm.zip file if it does not exist
    Invoke-WebRequest -Uri $Url -OutFile $ZipFile
}
# Check if nasm is already installed
if (!(Test-Path "$InstallDir\bin\nasm-2.16.01\nasm.exe")) {
    # Extract the contents of nasm.zip to the installation directory if it is not already extracted
    Expand-Archive -Path $ZipFile -DestinationPath $InstallDir\bin
}

# Add nasm to the PATH environment variable
$env:Path += ";$InstallDir\bin\nasm-2.16.01"

# Clone grpc
$GrpcDir = ".\tools\grpc"
if (!(Test-Path $GrpcDir -PathType Container)) {
    git clone -b v1.54.0 https://github.com/grpc/grpc.git .\tools\grpc
}

# Change to the grpc directory
Set-Location -Path .\tools\grpc

# Update submodules
git submodule update --init

# Apply the patch for boringssl if necessary
$PatchFile = "..\patch\0001-Fix-issue-with-Visual-Studio-2022-toolset.patch"
$BoringSSLDir = ".\third_party\boringssl-with-bazel"
Set-Location -Path $BoringSSLDir
if (!(git apply --check ..\..\$PatchFile)) {
    git apply $PatchFile
}
Set-Location -Path ..\..

# Check if the .build directory exists
$BuildDir = ".build"
if (!(Test-Path $BuildDir -PathType Container)) {
    # Create the .build directory if it does not exist
    New-Item -ItemType Directory -Name $BuildDir -Force | Out-Null
}

# Change to the .build directory
Set-Location -Path $BuildDir
Import-Module "$env:VS100COMNTOOLS\Microsoft.VisualStudio.DevShell.dll"

#Run CMake to generate Visual Studio configuration files
cmake -DCMAKE_INSTALL_PREFIX="$InstallDir" -DgRPC_INSTALL=ON -DgRPC_BUILD_TESTS=OFF .. -G "Visual Studio 17 2022"

#Compile the project in Release mode
cmake --build . --config Release
cmake --install . --config Release

#Compile the project in Debug mode
cmake --build . --config Debug
cmake --install . --config Debug
