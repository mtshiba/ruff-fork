from typing import NamedTuple
import typing

# with complex annotations
MyType = NamedTuple("MyType", [("a", int), ("b", tuple[str, ...])])

# with default values as list
MyType = NamedTuple(
    "MyType",
    [("a", int), ("b", str), ("c", list[bool])],
)

# with namespace
MyType = typing.NamedTuple("MyType", [("a", int), ("b", str)])

# too many default values (OK)
MyType = NamedTuple(
    "MyType",
    [("a", int), ("b", str)],
)

# invalid identifiers (OK)
MyType = NamedTuple("MyType", [("x-y", int), ("b", tuple[str, ...])])

# no fields
MyType = typing.NamedTuple("MyType")

# empty fields
MyType = typing.NamedTuple("MyType", [])

# keywords
MyType = typing.NamedTuple("MyType", a=int, b=tuple[str, ...])
