from abc import abstractmethod
from typing import overload, cast
from typing_extensions import override


###
# Unused arguments on functions.
###
def f(self, x):
    print("Hello, world!")


def f(cls, x):
    print("Hello, world!")


def f(self, x):
    ...


def f(cls, x):
    ...


###
# Unused arguments on lambdas.
###
lambda x: print("Hello, world!")

lambda: print("Hello, world!")


class C:
    ###
    # Unused arguments.
    ###
    def f(self, x):
        print("Hello, world!")

    def f(self, /, x):
        print("Hello, world!")

    def f(cls, x):
        print("Hello, world!")

    @classmethod
    def f(cls, x):
        print("Hello, world!")

    @staticmethod
    def f(cls, x):
        print("Hello, world!")

    @staticmethod
    def f(x):
        print("Hello, world!")

    ###
    # Unused arguments attached to empty functions (OK).
    ###
    def f(self, x):
        ...

    def f(self, /, x):
        ...

    def f(cls, x):
        ...

    @classmethod
    def f(cls, x):
        ...

    @staticmethod
    def f(cls, x):
        ...

    @staticmethod
    def f(x):
        ...

    def f(self, x):
        """Docstring."""

    def f(self, x):
        """Docstring."""
        ...

    def f(self, x):
        pass

    def f(self, x):
        raise NotImplementedError

    def f(self, x):
        raise NotImplementedError()

    def f(self, x):
        raise NotImplementedError("...")

    def f(self, x):
        raise NotImplemented

    def f(self, x):
        raise NotImplemented()

    def f(self, x):
        raise NotImplemented("...")

    ###
    # Unused functions attached to abstract methods (OK).
    ###
    @abstractmethod
    def f(self, x):
        print("Hello, world!")

    @abstractmethod
    def f(self, /, x):
        print("Hello, world!")

    @abstractmethod
    def f(cls, x):
        print("Hello, world!")

    @classmethod
    @abstractmethod
    def f(cls, x):
        print("Hello, world!")

    @staticmethod
    @abstractmethod
    def f(cls, x):
        print("Hello, world!")

    @staticmethod
    @abstractmethod
    def f(x):
        print("Hello, world!")

    ###
    # Unused functions attached to overrides (OK).
    ###
    @override
    def f(self, x):
        print("Hello, world!")

    @override
    def f(self, /, x):
        print("Hello, world!")

    @override
    def f(cls, x):
        print("Hello, world!")

    @classmethod
    @override
    def f(cls, x):
        print("Hello, world!")

    @staticmethod
    @override
    def f(cls, x):
        print("Hello, world!")

    @staticmethod
    @override
    def f(x):
        print("Hello, world!")


###
# Unused arguments attached to overloads (OK).
###
@overload
def f(a: str, b: str) -> str:
    ...


@overload
def f(a: int, b: int) -> str:
    ...


def f(a, b):
    return f"{a}{b}"


###
# Unused arguments on magic methods.
###
class C:
    def __init__(self, x) -> None:
        print("Hello, world!")

    def __str__(self) -> str:
        return "Hello, world!"

    def __exit__(self, exc_type, exc_value, traceback) -> None:
        print("Hello, world!")


###
# Used arguments on chained cast.
###
def f(x: None) -> None:
    _ = cast(Any, _identity)(x=x)

###
# Unused arguments with `locals`.
###
def f(bar: str):
    print(locals())


class C:
    def __init__(self, x) -> None:
        print(locals())

###
# Test with the different combinations of arguments
###

def multiple_posonly(a1, a2: int = 1, /, b: int = 1, *, d: int, **e: int):
    print(a1, b, d, e)


def last_posonly(a: int = 1, /, b: int = 1, *, d: int, **e: int):
    print(b, d, e)


def last_after_posonly(a: int = 1, /, c: int = 1):
    print(a)


def arg(a: int = 1, /, b: int = 1, *, d: int = 1, **e: int):
    print(a, d, e)


def vararg_and_kwonly(a: int = 1, /, b: int = 1, *c: int, d: int = 1, **e: int):
    print(a, b, d, e)


def vararg_and_kwargs(a: int = 1, /, b: int = 1, *c: int, **e: int):
    print(a, b, e)


def multiple_kwonly(a: int = 1, /, b: int = 1, *, d1: int = 1, d2: int = 1, **e: int):
    print(a, b, d1, e)


def last_kwonly_with_vararg(a: int = 1, /, b: int = 1, *c: int, d: int = 1, **e: int):
    print(a, b, c, e)


def last_kwonly_without_vararg(a: int = 1, /, b: int = 1, *, d: int = 1, **e: int):
    print(a, b, e)


def kwargs(a: int = 1, /, b: int = 1, *, d: int = 1, **e: int):
    print(a, b, d)


def only_posonly(a, /):
    ...


def only_arg(b):
    ...


def only_vararg(*c):
    ...


def only_kwonly(*, d):
    ...


def only_kwargs(**e):
    ...
