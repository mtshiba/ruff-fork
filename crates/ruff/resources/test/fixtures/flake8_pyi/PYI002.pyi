import sys

if sys.version_info == "1": # OK
  ...

if sys.platform <= "2": # OK
  ...

if sys.prefix == "1": # Must be one of "sys.version_info" or "sys.platform"
  ...

import some_module
if some_module.platform == "3": # Must be one of "sys.version_info" or "sys.platform"
  ...

if sys.version == "1" or sys.version_info == "2": # Number of comparitors must be 1
  ...

if sum([2,3]) > 4: # Comparison must be against an attribute
  ...

if bool(True): # If statement must contain a compare expression
  ...
