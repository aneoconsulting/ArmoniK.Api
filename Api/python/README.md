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

### **How to install the generated package**
From this directory, use the following command:
```code 
pip install pkg/armonik*.whl
```
The generated package will be installed to your current python environment

## Windows
Coming soon