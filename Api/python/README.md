# ArmoniK.Api Python package
This folder contains the necessary scripts to generate the ArmoniK.Api Python package. Please note that while the package generation is platform dependent, the generated package *should* be compatible with Linux and Windows.
## Linux
### **How to generate**
To generate the package from sources, run the proto2python.sh script.
You need to have python3-venv, python3, and pip installed. If the python command doesn't link to python3 on your system, you may be able to install the package python-is-python3.

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