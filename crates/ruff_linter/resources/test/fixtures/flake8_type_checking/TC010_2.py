from typing import TypeVar


x: "int" | str  # TC010
x: ("int" | str) | "bool"  # TC010
x: b"int" | str  # TC010 (unfixable)


def func():
    x: "int" | str  # OK


z: list[str, str | "int"] = []  # TC010

type A = Value["int" | str]  # OK

OldS = TypeVar('OldS', int | 'str', str)  # TC010

x: ("int"  # TC010 (unsafe fix)
    " | str" | None)
