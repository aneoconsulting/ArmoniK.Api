# ArmoniK.Api Python package
This folder contains the necessary scripts to generate the ArmoniK.Api Python package. Please note that while the package generation is platform dependent, the generated package *should* be compatible with Linux and Windows.

## Install the Python Package from PyPI
At each release, we provide a prepackaged version of ArmoniK.Api available on PyPI here : [https://pypi.org/project/armonik](https://pypi.org/project/armonik).
To install the package to your current Python environment, you can use pip :
```
pip install armonik
```

## Linux / [WSL](https://learn.microsoft.com/en-us/windows/wsl/)
### **How to generate**

Requirements :
- Python >= 3.7
- Python3-venv
- Pip3
- Bash

If the python command doesn't link to python3 on your system, you may be able to install the package python-is-python3, which links python to python3.

To generate the package from sources, run the [proto2python.sh](proto2python.sh) script from its folder. You need to specify a directory where the virtual environment used for the build will be located. For example the following command will generate the packages and will create the build environment "pyvenv" in the current user's home directory:

```bash
./proto2python.sh ~/pyvenv
```

3 folders will be created :
- generated : contains the source files used to create the package
- build : contains the source files used to create the wheel package
- pkg : contains the sdist and wheel packages

Instead of running the script you can install the armonik package in "editable" mode, allowing you to make changes to the source code, and the changes will be reflected immediately without reinstalling the package.

 ```bash 
 pip install -e ./packages/python
 ```

### **How to install the generated package**
From this directory, use the following command:
```code 
pip install pkg/armonik*.whl
```
The generated package will be installed to your current python environment

## Windows
Coming soon

## Tests

### **Test Environment Setup**

Before running tests, ensure the following setup steps are completed:

1. Install Dependencies:

```bash
sudo apt install dotnet-sdk-6.0 jq
```


2. Launch the Mock Server:

Verify that port 5000 is available (Armonik and the mock server communicate on the same port, uninstall armonik if necessary):

```bash
lsof -i :5000
```

3. Launch the server from the project's root directory in a separate terminal:

```bash
cd packages/csharp/ArmoniK.Api.Mock
dotnet run
```

### **Test Environment Summary**

The test environment utilizes a mock endpoint to assert if the ArmoniK service has been triggered. It leverages the requests library to query the /calls.json endpoint, examining the JSON response to validate the count of remote procedure calls made to specific services and methods

```bash
curl localhost:5000/calls.json | jq
```
