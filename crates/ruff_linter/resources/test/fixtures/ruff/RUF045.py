from dataclasses import InitVar, KW_ONLY, MISSING, dataclass, field
from typing import ClassVar


@dataclass
class C:
    # Errors
    no_annotation = r"foo"
    missing = MISSING
    field = field()

    # No errors
    with_annotation: str
    with_annotation_and_default: int = 42
    with_annotation_and_field_specifier: bytes = field()

    class_var_no_arguments: ClassVar = 42
    class_var_with_arguments: ClassVar[int] = 42

    init_var_no_arguments: InitVar = "lorem"
    init_var_with_arguments: InitVar[str] = "ipsum"

    kw_only: KW_ONLY
    multiple, targets = (0, 1)
