# Set directory for installation - Chocolatey does not lock
# down the directory if not the default
$InstallDir="$pwd\tools\win64"
$env:VS100COMNTOOLS="C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools"

# Vérifier si le répertoire d'installation de nasm existe
if (!(Test-Path $InstallDir -PathType Container)) {
    # Créer le répertoire d'installation de nasm s'il n'existe pas
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


# Installer CMake
choco install -y cmake.portable --force --installargs "ALLUSERS=0 INSTALLDIR=$InstallDir MSIINSTALLPERUSER=1 "

# Télécharger et extraire nasm
$Url = "https://www.nasm.us/pub/nasm/releasebuilds/2.16.01/win64/nasm-2.16.01-win64.zip"
$ZipFile = "$InstallDir\nasm.zip"

# Vérifier si le fichier zip existe
if (!(Test-Path $ZipFile -PathType Leaf)) {
    # Télécharger le fichier nasm.zip s'il n'existe pas
    Invoke-WebRequest -Uri $Url -OutFile $ZipFile
}
# Vérifier si nasm est déjà installé
if (!(Test-Path "$InstallDir\bin\nasm-2.16.01\nasm.exe")) {
    # Extraire le contenu de nasm.zip dans le répertoire d'installation s'il n'est pas déjà extrait
    Expand-Archive -Path $ZipFile -DestinationPath $InstallDir\bin
}

# Ajouter nasm à la variable d'environnement PATH
$env:Path += ";$InstallDir\bin\nasm-2.16.01"

# Cloner grpc
$GrpcDir = ".\tools\grpc"
if (!(Test-Path $GrpcDir -PathType Container)) {
    git clone -b v1.54.0 https://github.com/grpc/grpc.git .\tools\grpc
}

# Accéder au répertoire grpc
Set-Location -Path .\tools\grpc

# Mettre à jour les sous-modules
git submodule update --init

# Appliquer le patch pour boringssl si nécessaire
$PatchFile = "..\patch\0001-Fix-issue-with-Visual-Studio-2022-toolset.patch"
$BoringSSLDir = ".\third_party\boringssl-with-bazel"
Set-Location -Path $BoringSSLDir
if (!(git apply --check ..\..\$PatchFile)) {
    git apply $PatchFile
}
Set-Location -Path ..\..

# Vérifier si le répertoire .build existe
$BuildDir = ".build"
if (!(Test-Path $BuildDir -PathType Container)) {
    # Créer le répertoire .build s'il n'existe pas
    New-Item -ItemType Directory -Name $BuildDir -Force | Out-Null
}

# Accéder au répertoire .build
Set-Location -Path $BuildDir
Import-Module "$env:VS100COMNTOOLS\Microsoft.VisualStudio.DevShell.dll"

# Lancer CMake pour générer les fichiers de configuration de Visual Studio
cmake -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$InstallDir" -DgRPC_INSTALL=ON -DgRPC_BUILD_TESTS=OFF .. -G "Visual Studio 17 2022"

# Compiler le projet en mode Release
cmake --build . --config Release
cmake --install . --config Release

# Compiler le projet en mode Release
cmake --build . --config Debug
cmake --install . --config Debug