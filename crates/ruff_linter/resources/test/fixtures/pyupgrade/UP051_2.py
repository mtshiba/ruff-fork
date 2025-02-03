# bound
class Foo[_T: str]:
    var: _T


# constraint
class Foo[_T: (str, bytes)]:
    var: _T


# tuple
class Foo[*_Ts]:
    var: tuple[*_Ts]


# paramspec
class C[**_P]:
    var: _P


from typing import Callable


# each of these will get a separate diagnostic, but at least they'll all get
# fixed
class Everything[_T, _U: str, _V: (int, float), *_W, **_X]:
    @staticmethod
    def transform(t: _T, u: _U, v: _V) -> tuple[*_W] | Callable[_X, _T] | None:
        return None
