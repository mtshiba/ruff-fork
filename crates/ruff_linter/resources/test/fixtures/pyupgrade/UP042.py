from enum import Enum

class A(str, Enum):
    ...

class B(Enum, str):
    ...

class D(int, str, Enum):
    ...

class E(str, int, Enum):
    ...

# TODO: add more cases
