class Test(
    Aaaaaaaaaaaaaaaaa,
    Bbbbbbbbbbbbbbbb,
    DDDDDDDDDDDDDDDD,
    EEEEEEEEEEEEEE,
    metaclass=meta,
):
    pass


class Test((Aaaaaaaaaaaaaaaaa), Bbbbbbbbbbbbbbbb, metaclass=meta):
    pass

class Test( # trailing class comment
    Aaaaaaaaaaaaaaaaa, # trailing comment

    # in between comment

    Bbbbbbbbbbbbbbbb,
    # another leading comment
    DDDDDDDDDDDDDDDD,
    EEEEEEEEEEEEEE,
    # meta comment
    metaclass=meta, # trailing meta comment
):
    pass

class Test((Aaaa)):
    ...


class Test(aaaaaaaaaaaaaaa + bbbbbbbbbbbbbbbbbbbbbb + cccccccccccccccccccccccc + dddddddddddddddddddddd + eeeeeeeee, ffffffffffffffffff, gggggggggggggggggg):
    pass

class Test(aaaaaaaaaaaaaaa + bbbbbbbbbbbbbbbbbbbbbb * cccccccccccccccccccccccc + dddddddddddddddddddddd + eeeeeeeee, ffffffffffffffffff, gggggggggggggggggg):
    pass

class TestTrailingComment1(Aaaa): # trailing comment
    pass


class TestTrailingComment2: # trailing comment
    pass


class Test:
    """Docstring"""


class Test:
    # comment
    """Docstring"""


class Test:
    """Docstring"""
    x = 1


class Test:
    """Docstring"""
    # comment
    x = 1


class Test:

    """Docstring"""


class Test:
    # comment

    """Docstring"""


class Test:

    # comment

    """Docstring"""


class Test:

    """Docstring"""
    x = 1


class Test:

    """Docstring"""
    # comment
    x = 1


class C(): # comment
    pass


class C(  # comment
):
    pass


class C(
    # comment
):
    pass


class C(): # comment
    pass


class C(  # comment
    # comment
    1
):
    pass


class C(
    1
    # comment
):
    pass


@dataclass
# Copied from transformers.models.clip.modeling_clip.CLIPOutput with CLIP->AltCLIP
class AltCLIPOutput(ModelOutput):
    ...


@dataclass
class AltCLIPOutput( # Copied from transformers.models.clip.modeling_clip.CLIPOutput with CLIP->AltCLIP
):
    ...


@dataclass
class AltCLIPOutput(
    # Copied from transformers.models.clip.modeling_clip.CLIPOutput with CLIP->AltCLIP
):
    ...
