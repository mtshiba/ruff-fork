from typing import Final, Literal

x: Final[Literal[True]] = True  # PYI064
y: Final[Literal[None]] = None  # PYI064
z: Final[Literal[  # PYI064
    "this is a really long literal, that won't be rendered in the issue text"
]] = "this is a really long literal, that won't be rendered in the issue text"

# This should be fixable, and marked as safe
w1: Final[Literal[123]]  # PYI064

# This should be fixable, but marked as unsafe
w2: Final[Literal[123]] = "random value"  # PYI064

n1: Final[Literal[True, False]] = True # No issue here
n2: Literal[True] = True  # No issue here
