# These testcases should raise errors

class Float:
    def __bytes__(self):
        return 3.05  # [invalid-bytes-return]

class Int:
    def __bytes__(self):
        return 0  # [invalid-bytes-return]

class Str:
    def __bytes__(self):
        return "some bytes"  # [invalid-bytes-return]

# TODO: Once Ruff has better type checking
def return_bytes():
    return "some string"

class ComplexReturn:
    def __bytes__(self):
        return return_bytes()  # [invalid-bytes-return]



# These testcases should NOT raise errors

class Bytes:
    def __bytes__(self):
        return b"some bytes"


class Bytes2:
    def __bytes__(self):
        x = b"some bytes"
        return x
