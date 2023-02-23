"""Vendored from [scripts/mkstdlibs.py in PyCQA/isort](https://github.com/PyCQA/isort/blob/e321a670d0fefdea0e04ed9d8d696434cf49bdec/scripts/mkstdlibs.py).

Only the generation of the file has been modified for use in this project.
"""

from pathlib import Path

from sphinx.ext.intersphinx import fetch_inventory

URL = "https://docs.python.org/{}/objects.inv"
PATH = Path("crates") / "ruff_python" / "src" / "sys.rs"
VERSIONS: list[tuple[int, int]] = [
    (3, 7),
    (3, 8),
    (3, 9),
    (3, 10),
    (3, 11),
]


class FakeConfig:  # noqa: D101
    intersphinx_timeout = None
    tls_verify = True
    user_agent = ""


class FakeApp:  # noqa: D101
    srcdir = ""
    config = FakeConfig()


with PATH.open("w") as f:
    f.write(
        """\
//! This file is generated by `scripts/generate_known_standard_library.py`
use once_cell::sync::Lazy;
use rustc_hash::{FxHashMap, FxHashSet};

// See: https://pycqa.github.io/isort/docs/configuration/options.html#known-standard-library
pub static KNOWN_STANDARD_LIBRARY: Lazy<FxHashMap<(u32, u32), FxHashSet<&'static str>>> =
    Lazy::new(|| {
        FxHashMap::from_iter([
""",
    )
    for major, minor in VERSIONS:
        version = f"{major}.{minor}"
        url = URL.format(version)
        invdata = fetch_inventory(FakeApp(), "", url)

        # Any modules we want to enforce across Python versions stdlib can be included in set init
        modules = {
            "_ast",
            "posixpath",
            "ntpath",
            "sre_constants",
            "sre_parse",
            "sre_compile",
            "sre",
        }
        for module in invdata["py:module"]:
            root, *_ = module.split(".")
            if root not in ["__future__", "__main__"]:
                modules.add(root)

        f.write(
            f"""\
            (
                ({major}, {minor}),
                FxHashSet::from_iter([
""",
        )
        for module in sorted(modules):
            f.write(
                f"""\
                    "{module}",
""",
            )
        f.write(
            """\
                ]),
            ),
""",
        )
    f.write(
        """\
        ])
    });
""",
    )
