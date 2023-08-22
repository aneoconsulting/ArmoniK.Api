import json
import subprocess
from typing import List, Tuple

from dataclasses import dataclass

@dataclass
class RPC:
    # Name of the service
    service: str
    # Name of the procedure
    procedure: str
    # Is the procedure deprecated and by what is it replaced ?
    deprecated: bool = False

    def __eq__(self, other) -> bool:
        return self.service == other.service and self.procedure == other.procedure
    
    def __str__(self) -> str:
        return f"RPC(\"{self.service}\", \"{self.procedure}\", {self.deprecated})"

####################

# Deprecated RPCs
depreciation_list : List[RPC] = [
    RPC("Submitter", "CreateSmallTasks")
    ]

####################

header = """from dataclasses import dataclass

@dataclass
class RPC:
    # Name of the service
    service: str
    # Name of the procedure
    procedure: str
    # Is the procedure deprecated and by what is it replaced ?
    deprecated: bool = False

    def __eq__(self, other) -> bool:
        return self.service == other.service and self.procedure == other.procedure
    
    def __str__(self) -> str:
        return f"RPC(\\\"{self.service}\\\", \\\"{self.procedure}\\\", {self.deprecated})"

"""

def main(address = "localhost:5000/calls.json"):
    # Pings the mock to get list of rpcs and generate the rpcs.py file
    value = subprocess.run(["curl", address], check=True, capture_output=True)
    json_data : dict[str, dict[str, int]] = json.loads(value.stdout)
    with open("rpcs.py", "w") as f:
        f.write(header)
        rpc_classes = []
        for cls_name, methods in json_data.items():
            cls_lines = []
            cls_lines.append(f"class {cls_name}:")
            for method_name in methods:
                rpc = RPC(cls_name, method_name)
                rpc.deprecated = rpc in depreciation_list
                cls_lines.append(f"    {method_name} = {str(rpc)}")
            rpc_classes.append("\n".join(cls_lines))
        f.write("\n\n".join(rpc_classes))
    print("rpcs.py written")

if __name__ == "__main__":
    main()