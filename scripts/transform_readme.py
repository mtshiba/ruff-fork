"""Transform the README.md to support a specific deployment target.

By default, we assume that our README.md will be rendered on GitHub. However, different
targets have different strategies for rendering light- and dark-mode images. This script
adjusts the images in the README.md to support the given target.
"""
import argparse
from pathlib import Path

URL = "https://user-images.githubusercontent.com/1309177/{}.svg"
URL_LIGHT = URL.format("212613257-5f4bca12-6d6b-4c79-9bac-51a4c6d08928")
URL_DARK = URL.format("212613422-7faaf278-706b-4294-ad92-236ffcab3430")

# https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#specifying-the-theme-an-image-is-shown-to
GITHUB = f"""
<p align="center">
  <picture align="center">
    <source media="(prefers-color-scheme: dark)" srcset="{URL_DARK}">
    <source media="(prefers-color-scheme: light)" srcset="{URL_LIGHT}">
    <img alt="Shows a bar chart with benchmark results." src="{URL_LIGHT}">
  </picture>
</p>
"""

# https://github.com/pypi/warehouse/issues/11251
PYPI = f"""
<p align="center">
  <img alt="Shows a bar chart with benchmark results." src="{URL_LIGHT}">
</p>
"""

# The CSS classes are defined in docs/theme/exclude.css
MDBOOK = f"""
<p align="center">
  <img alt="Shows a bar chart with benchmark results." src="{URL_LIGHT}" class="no-dark-themes">
</p>

<p align="center">
  <img alt="Shows a bar chart with benchmark results." src="{URL_DARK} class="no-light-themes">
</p>
"""


def main(target: str) -> None:
    """Modify the README.md to support the given target."""
    with Path("README.md").open(encoding="utf8") as fp:
        content = fp.read()
        if GITHUB not in content:
            msg = "README.md is not in the expected format."
            raise ValueError(msg)

    if target == "pypi":
        with Path("README.md").open("w", encoding="utf8") as fp:
            fp.write(content.replace(GITHUB, PYPI))
    elif target == "mdbook":
        with Path("README.md").open("w", encoding="utf8") as fp:
            fp.write(content.replace(GITHUB, MDBOOK))
    else:
        msg = f"Unknown target: {target}"
        raise ValueError(msg)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Modify the README.md to support a specific deployment target.",
    )
    parser.add_argument(
        "--target",
        type=str,
        required=True,
        choices=("pypi", "mdbook"),
    )
    args = parser.parse_args()

    main(target=args.target)
