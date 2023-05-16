bla = b"bla"
d = {"a": b"bla", "b": b"bla", "c": b"bla"}


def foo(one_arg):
    pass


f"{str(bla)}, {repr(bla)}, {ascii(bla)}"  # RUF010

f"{str(d['a'])}, {repr(d['b'])}, {ascii(d['c'])}"  # RUF010

f"{foo(bla)}"  # OK

f"{str(bla, 'ascii')}, {str(bla, encoding='cp1255')}"  # OK

f"{bla!s} {[]!r} {'bar'!a}"  # OK

"Not an f-string {str(bla)}, {repr(bla)}, {ascii(bla)}"  # OK


def ascii(arg):
    pass


f"{ascii(bla)}"  # OK
