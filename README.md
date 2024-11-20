<!-- Begin section: Overview -->

# Ruff

[![Ruff](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json)](https://github.com/astral-sh/ruff)
[![image](https://img.shields.io/pypi/v/ruff.svg)](https://pypi.python.org/pypi/ruff)
[![image](https://img.shields.io/pypi/l/ruff.svg)](https://github.com/astral-sh/ruff/blob/main/LICENSE)
[![image](https://img.shields.io/pypi/pyversions/ruff.svg)](https://pypi.python.org/pypi/ruff)
[![Actions status](https://github.com/astral-sh/ruff/workflows/CI/badge.svg)](https://github.com/astral-sh/ruff/actions)
[![Discord](https://img.shields.io/badge/Discord-%235865F2.svg?logo=discord&logoColor=white)](https://discord.com/invite/astral-sh)

[**Docs**](https://docs.astral.sh/ruff/) | [**Playground**](https://play.ruff.rs/)

An extremely fast Python linter and code formatter, written in Rust.

<p align="center">
  <picture align="center">
    <source media="(prefers-color-scheme: dark)" srcset="https://user-images.githubusercontent.com/1309177/232603514-c95e9b0f-6b31-43de-9a80-9e844173fd6a.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://user-images.githubusercontent.com/1309177/232603516-4fb4892d-585c-4b20-b810-3db9161831e4.svg">
    <img alt="Shows a bar chart with benchmark results." src="https://user-images.githubusercontent.com/1309177/232603516-4fb4892d-585c-4b20-b810-3db9161831e4.svg">
  </picture>
</p>

<p align="center">
  <i>Linting the CPython codebase from scratch.</i>
</p>

- ⚡️ 10-100x faster than existing linters (like Flake8) and formatters (like Black)
- 🐍 Installable via `pip`
- 🛠️ `pyproject.toml` support
- 🤝 Python 3.13 compatibility
- ⚖️ Drop-in parity with [Flake8](https://docs.astral.sh/ruff/faq/#how-does-ruffs-linter-compare-to-flake8), isort, and [Black](https://docs.astral.sh/ruff/faq/#how-does-ruffs-formatter-compare-to-black)
- 📦 Built-in caching, to avoid re-analyzing unchanged files
- 🔧 Fix support, for automatic error correction (e.g., automatically remove unused imports)
- 📏 Over [800 built-in rules](https://docs.astral.sh/ruff/rules/), with native re-implementations
    of popular Flake8 plugins, like flake8-bugbear
- ⌨️ First-party [editor integrations](https://docs.astral.sh/ruff/integrations/) for
    [VS Code](https://github.com/astral-sh/ruff-vscode) and [more](https://docs.astral.sh/ruff/editors/setup)
- 🌎 Monorepo-friendly, with [hierarchical and cascading configuration](https://docs.astral.sh/ruff/configuration/#config-file-discovery)

Ruff aims to be orders of magnitude faster than alternative tools while integrating more
functionality behind a single, common interface.

Ruff can be used to replace [Flake8](https://pypi.org/project/flake8/) (plus dozens of plugins),
[Black](https://github.com/psf/black), [isort](https://pypi.org/project/isort/),
[pydocstyle](https://pypi.org/project/pydocstyle/), [pyupgrade](https://pypi.org/project/pyupgrade/),
[autoflake](https://pypi.org/project/autoflake/), and more, all while executing tens or hundreds of
times faster than any individual tool.

Ruff is extremely actively developed and used in major open-source projects like:

- [Apache Airflow](https://github.com/apache/airflow)
- [Apache Superset](https://github.com/apache/superset)
- [FastAPI](https://github.com/tiangolo/fastapi)
- [Hugging Face](https://github.com/huggingface/transformers)
- [Pandas](https://github.com/pandas-dev/pandas)
- [SciPy](https://github.com/scipy/scipy)

...and [many more](#whos-using-ruff).

Ruff is backed by [Astral](https://astral.sh). Read the [launch post](https://astral.sh/blog/announcing-astral-the-company-behind-ruff),
or the original [project announcement](https://notes.crmarsh.com/python-tooling-could-be-much-much-faster).

## Testimonials

[**Sebastián Ramírez**](https://twitter.com/tiangolo/status/1591912354882764802), creator
of [FastAPI](https://github.com/tiangolo/fastapi):

> Ruff is so fast that sometimes I add an intentional bug in the code just to confirm it's actually
> running and checking the code.

[**Nick Schrock**](https://twitter.com/schrockn/status/1612615862904827904), founder of [Elementl](https://www.elementl.com/),
co-creator of [GraphQL](https://graphql.org/):

> Why is Ruff a gamechanger? Primarily because it is nearly 1000x faster. Literally. Not a typo. On
> our largest module (dagster itself, 250k LOC) pylint takes about 2.5 minutes, parallelized across 4
> cores on my M1. Running ruff against our _entire_ codebase takes .4 seconds.

[**Bryan Van de Ven**](https://github.com/bokeh/bokeh/pull/12605), co-creator
of [Bokeh](https://github.com/bokeh/bokeh/), original author
of [Conda](https://docs.conda.io/en/latest/):

> Ruff is ~150-200x faster than flake8 on my machine, scanning the whole repo takes ~0.2s instead of
> ~20s. This is an enormous quality of life improvement for local dev. It's fast enough that I added
> it as an actual commit hook, which is terrific.

[**Timothy Crosley**](https://twitter.com/timothycrosley/status/1606420868514877440),
creator of [isort](https://github.com/PyCQA/isort):

> Just switched my first project to Ruff. Only one downside so far: it's so fast I couldn't believe
> it was working till I intentionally introduced some errors.

[**Tim Abbott**](https://github.com/astral-sh/ruff/issues/465#issuecomment-1317400028), lead
developer of [Zulip](https://github.com/zulip/zulip):

> This is just ridiculously fast... `ruff` is amazing.

<!-- End section: Overview -->

## Table of Contents

For more, see the [documentation](https://docs.astral.sh/ruff/).

1. [Getting Started](#getting-started)
1. [Configuration](#configuration)
1. [Rules](#rules)
1. [Contributing](#contributing)
1. [Support](#support)
1. [Acknowledgements](#acknowledgements)
1. [Who's Using Ruff?](#whos-using-ruff)
1. [License](#license)

## Getting Started<a id="getting-started"></a>

For more, see the [documentation](https://docs.astral.sh/ruff/).

### Installation

Ruff is available as [`ruff`](https://pypi.org/project/ruff/) on PyPI:

```shell
# With pip.
pip install ruff

# With pipx.
pipx install ruff
```

Starting with version `0.5.0`, Ruff can be installed with our standalone installers:

```shell
# On macOS and Linux.
curl -LsSf https://astral.sh/ruff/install.sh | sh

# On Windows.
powershell -c "irm https://astral.sh/ruff/install.ps1 | iex"

# For a specific version.
curl -LsSf https://astral.sh/ruff/0.7.4/install.sh | sh
powershell -c "irm https://astral.sh/ruff/0.7.4/install.ps1 | iex"
```

You can also install Ruff via [Homebrew](https://formulae.brew.sh/formula/ruff), [Conda](https://anaconda.org/conda-forge/ruff),
and with [a variety of other package managers](https://docs.astral.sh/ruff/installation/).

### Usage

To run Ruff as a linter, try any of the following:

```shell
ruff check                          # Lint all files in the current directory (and any subdirectories).
ruff check path/to/code/            # Lint all files in `/path/to/code` (and any subdirectories).
ruff check path/to/code/*.py        # Lint all `.py` files in `/path/to/code`.
ruff check path/to/code/to/file.py  # Lint `file.py`.
ruff check @arguments.txt           # Lint using an input file, treating its contents as newline-delimited command-line arguments.
```

Or, to run Ruff as a formatter:

```shell
ruff format                          # Format all files in the current directory (and any subdirectories).
ruff format path/to/code/            # Format all files in `/path/to/code` (and any subdirectories).
ruff format path/to/code/*.py        # Format all `.py` files in `/path/to/code`.
ruff format path/to/code/to/file.py  # Format `file.py`.
ruff format @arguments.txt           # Format using an input file, treating its contents as newline-delimited command-line arguments.
```

Ruff can also be used as a [pre-commit](https://pre-commit.com/) hook via [`ruff-pre-commit`](https://github.com/astral-sh/ruff-pre-commit):

```yaml
- repo: https://github.com/astral-sh/ruff-pre-commit
  # Ruff version.
  rev: v0.7.4
  hooks:
    # Run the linter.
    - id: ruff
      args: [ --fix ]
    # Run the formatter.
    - id: ruff-format
```

Ruff can also be used as a [VS Code extension](https://github.com/astral-sh/ruff-vscode) or with [various other editors](https://docs.astral.sh/ruff/editors/setup).

Ruff can also be used as a [GitHub Action](https://github.com/features/actions) via
[`ruff-action`](https://github.com/astral-sh/ruff-action):

```yaml
name: Ruff
on: [ push, pull_request ]
jobs:
  ruff:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: astral-sh/ruff-action@v1
```

### Configuration<a id="configuration"></a>

Ruff can be configured through a `pyproject.toml`, `ruff.toml`, or `.ruff.toml` file (see:
[_Configuration_](https://docs.astral.sh/ruff/configuration/), or [_Settings_](https://docs.astral.sh/ruff/settings/)
for a complete list of all configuration options).

If left unspecified, Ruff's default configuration is equivalent to the following `ruff.toml` file:

```toml
# Exclude a variety of commonly ignored directories.
exclude = [
    ".bzr",
    ".direnv",
    ".eggs",
    ".git",
    ".git-rewrite",
    ".hg",
    ".ipynb_checkpoints",
    ".mypy_cache",
    ".nox",
    ".pants.d",
    ".pyenv",
    ".pytest_cache",
    ".pytype",
    ".ruff_cache",
    ".svn",
    ".tox",
    ".venv",
    ".vscode",
    "__pypackages__",
    "_build",
    "buck-out",
    "build",
    "dist",
    "node_modules",
    "site-packages",
    "venv",
]

# Same as Black.
line-length = 88
indent-width = 4

# Assume Python 3.9
target-version = "py39"

[lint]
# Enable Pyflakes (`F`) and a subset of the pycodestyle (`E`)  codes by default.
select = ["E4", "E7", "E9", "F"]
ignore = []

# Allow fix for all enabled rules (when `--fix`) is provided.
fixable = ["ALL"]
unfixable = []

# Allow unused variables when underscore-prefixed.
dummy-variable-rgx = "^(_+|(_+[a-zA-Z0-9_]*[a-zA-Z0-9]+?))$"

[format]
# Like Black, use double quotes for strings.
quote-style = "double"

# Like Black, indent with spaces, rather than tabs.
indent-style = "space"

# Like Black, respect magic trailing commas.
skip-magic-trailing-comma = false

# Like Black, automatically detect the appropriate line ending.
line-ending = "auto"
```

Note that, in a `pyproject.toml`, each section header should be prefixed with `tool.ruff`. For
example, `[lint]` should be replaced with `[tool.ruff.lint]`.

Some configuration options can be provided via dedicated command-line arguments, such as those
related to rule enablement and disablement, file discovery, and logging level:

```shell
ruff check --select F401 --select F403 --quiet
```

The remaining configuration options can be provided through a catch-all `--config` argument:

```shell
ruff check --config "lint.per-file-ignores = {'some_file.py' = ['F841']}"
```

To opt in to the latest lint rules, formatter style changes, interface updates, and more, enable
[preview mode](https://docs.astral.sh/ruff/rules/) by setting `preview = true` in your configuration
file or passing `--preview` on the command line. Preview mode enables a collection of unstable
features that may change prior to stabilization.

See `ruff help` for more on Ruff's top-level commands, or `ruff help check` and `ruff help format`
for more on the linting and formatting commands, respectively.

## Rules<a id="rules"></a>

<!-- Begin section: Rules -->

**Ruff supports over 800 lint rules**, many of which are inspired by popular tools like Flake8,
isort, pyupgrade, and others. Regardless of the rule's origin, Ruff re-implements every rule in
Rust as a first-party feature.

By default, Ruff enables Flake8's `F` rules, along with a subset of the `E` rules, omitting any
stylistic rules that overlap with the use of a formatter, like `ruff format` or
[Black](https://github.com/psf/black).

If you're just getting started with Ruff, **the default rule set is a great place to start**: it
catches a wide variety of common errors (like unused imports) with zero configuration.

<!-- End section: Rules -->

Beyond the defaults, Ruff re-implements some of the most popular Flake8 plugins and related code
quality tools, including:

- [autoflake](https://pypi.org/project/autoflake/)
- [eradicate](https://pypi.org/project/eradicate/)
- [flake8-2020](https://pypi.org/project/flake8-2020/)
- [flake8-annotations](https://pypi.org/project/flake8-annotations/)
- [flake8-async](https://pypi.org/project/flake8-async)
- [flake8-bandit](https://pypi.org/project/flake8-bandit/) ([#1646](https://github.com/astral-sh/ruff/issues/1646))
- [flake8-blind-except](https://pypi.org/project/flake8-blind-except/)
- [flake8-boolean-trap](https://pypi.org/project/flake8-boolean-trap/)
- [flake8-bugbear](https://pypi.org/project/flake8-bugbear/)
- [flake8-builtins](https://pypi.org/project/flake8-builtins/)
- [flake8-commas](https://pypi.org/project/flake8-commas/)
- [flake8-comprehensions](https://pypi.org/project/flake8-comprehensions/)
- [flake8-copyright](https://pypi.org/project/flake8-copyright/)
- [flake8-datetimez](https://pypi.org/project/flake8-datetimez/)
- [flake8-debugger](https://pypi.org/project/flake8-debugger/)
- [flake8-django](https://pypi.org/project/flake8-django/)
- [flake8-docstrings](https://pypi.org/project/flake8-docstrings/)
- [flake8-eradicate](https://pypi.org/project/flake8-eradicate/)
- [flake8-errmsg](https://pypi.org/project/flake8-errmsg/)
- [flake8-executable](https://pypi.org/project/flake8-executable/)
- [flake8-future-annotations](https://pypi.org/project/flake8-future-annotations/)
- [flake8-gettext](https://pypi.org/project/flake8-gettext/)
- [flake8-implicit-str-concat](https://pypi.org/project/flake8-implicit-str-concat/)
- [flake8-import-conventions](https://github.com/joaopalmeiro/flake8-import-conventions)
- [flake8-logging](https://pypi.org/project/flake8-logging/)
- [flake8-logging-format](https://pypi.org/project/flake8-logging-format/)
- [flake8-no-pep420](https://pypi.org/project/flake8-no-pep420)
- [flake8-pie](https://pypi.org/project/flake8-pie/)
- [flake8-print](https://pypi.org/project/flake8-print/)
- [flake8-pyi](https://pypi.org/project/flake8-pyi/)
- [flake8-pytest-style](https://pypi.org/project/flake8-pytest-style/)
- [flake8-quotes](https://pypi.org/project/flake8-quotes/)
- [flake8-raise](https://pypi.org/project/flake8-raise/)
- [flake8-return](https://pypi.org/project/flake8-return/)
- [flake8-self](https://pypi.org/project/flake8-self/)
- [flake8-simplify](https://pypi.org/project/flake8-simplify/)
- [flake8-slots](https://pypi.org/project/flake8-slots/)
- [flake8-super](https://pypi.org/project/flake8-super/)
- [flake8-tidy-imports](https://pypi.org/project/flake8-tidy-imports/)
- [flake8-todos](https://pypi.org/project/flake8-todos/)
- [flake8-type-checking](https://pypi.org/project/flake8-type-checking/)
- [flake8-use-pathlib](https://pypi.org/project/flake8-use-pathlib/)
- [flynt](https://pypi.org/project/flynt/) ([#2102](https://github.com/astral-sh/ruff/issues/2102))
- [isort](https://pypi.org/project/isort/)
- [mccabe](https://pypi.org/project/mccabe/)
- [pandas-vet](https://pypi.org/project/pandas-vet/)
- [pep8-naming](https://pypi.org/project/pep8-naming/)
- [pydocstyle](https://pypi.org/project/pydocstyle/)
- [pygrep-hooks](https://github.com/pre-commit/pygrep-hooks)
- [pylint-airflow](https://pypi.org/project/pylint-airflow/)
- [pyupgrade](https://pypi.org/project/pyupgrade/)
- [tryceratops](https://pypi.org/project/tryceratops/)
- [yesqa](https://pypi.org/project/yesqa/)

For a complete enumeration of the supported rules, see [_Rules_](https://docs.astral.sh/ruff/rules/).

## Contributing<a id="contributing"></a>

Contributions are welcome and highly appreciated. To get started, check out the
[**contributing guidelines**](https://docs.astral.sh/ruff/contributing/).

You can also join us on [**Discord**](https://discord.com/invite/astral-sh).

## Support<a id="support"></a>

Having trouble? Check out the existing issues on [**GitHub**](https://github.com/astral-sh/ruff/issues),
or feel free to [**open a new one**](https://github.com/astral-sh/ruff/issues/new).

You can also ask for help on [**Discord**](https://discord.com/invite/astral-sh).

## Acknowledgements<a id="acknowledgements"></a>

Ruff's linter draws on both the APIs and implementation details of many other
tools in the Python ecosystem, especially [Flake8](https://github.com/PyCQA/flake8), [Pyflakes](https://github.com/PyCQA/pyflakes),
[pycodestyle](https://github.com/PyCQA/pycodestyle), [pydocstyle](https://github.com/PyCQA/pydocstyle),
[pyupgrade](https://github.com/asottile/pyupgrade), and [isort](https://github.com/PyCQA/isort).

In some cases, Ruff includes a "direct" Rust port of the corresponding tool.
We're grateful to the maintainers of these tools for their work, and for all
the value they've provided to the Python community.

Ruff's formatter is built on a fork of Rome's [`rome_formatter`](https://github.com/rome/tools/tree/main/crates/rome_formatter),
and again draws on both API and implementation details from [Rome](https://github.com/rome/tools),
[Prettier](https://github.com/prettier/prettier), and [Black](https://github.com/psf/black).

Ruff's import resolver is based on the import resolution algorithm from [Pyright](https://github.com/microsoft/pyright).

Ruff is also influenced by a number of tools outside the Python ecosystem, like
[Clippy](https://github.com/rust-lang/rust-clippy) and [ESLint](https://github.com/eslint/eslint).

Ruff is the beneficiary of a large number of [contributors](https://github.com/astral-sh/ruff/graphs/contributors).

Ruff is released under the MIT license.

## Who's Using Ruff?<a id="whos-using-ruff"></a>

Ruff is used by a number of major open-source projects and companies, including:

- [Albumentations](https://github.com/albumentations-team/albumentations)
- Amazon ([AWS SAM](https://github.com/aws/serverless-application-model))
- Anthropic ([Python SDK](https://github.com/anthropics/anthropic-sdk-python))
- [Apache Airflow](https://github.com/apache/airflow)
- AstraZeneca ([Magnus](https://github.com/AstraZeneca/magnus-core))
- [Babel](https://github.com/python-babel/babel)
- Benchling ([Refac](https://github.com/benchling/refac))
- [Bokeh](https://github.com/bokeh/bokeh)
- CrowdCent ([NumerBlox](https://github.com/crowdcent/numerblox)) <!-- typos: ignore -->
- [Cryptography (PyCA)](https://github.com/pyca/cryptography)
- CERN ([Indico](https://getindico.io/))
- [DVC](https://github.com/iterative/dvc)
- [Dagger](https://github.com/dagger/dagger)
- [Dagster](https://github.com/dagster-io/dagster)
- Databricks ([MLflow](https://github.com/mlflow/mlflow))
- [Dify](https://github.com/langgenius/dify)
- [FastAPI](https://github.com/tiangolo/fastapi)
- [Godot](https://github.com/godotengine/godot)
- [Gradio](https://github.com/gradio-app/gradio)
- [Great Expectations](https://github.com/great-expectations/great_expectations)
- [HTTPX](https://github.com/encode/httpx)
- [Hatch](https://github.com/pypa/hatch)
- [Home Assistant](https://github.com/home-assistant/core)
- Hugging Face ([Transformers](https://github.com/huggingface/transformers),
    [Datasets](https://github.com/huggingface/datasets),
    [Diffusers](https://github.com/huggingface/diffusers))
- IBM ([Qiskit](https://github.com/Qiskit/qiskit))
- ING Bank ([popmon](https://github.com/ing-bank/popmon), [probatus](https://github.com/ing-bank/probatus))
- [Ibis](https://github.com/ibis-project/ibis)
- [ivy](https://github.com/unifyai/ivy)
- [Jupyter](https://github.com/jupyter-server/jupyter_server)
- [Kraken Tech](https://kraken.tech/)
- [LangChain](https://github.com/hwchase17/langchain)
- [Litestar](https://litestar.dev/)
- [LlamaIndex](https://github.com/jerryjliu/llama_index)
- Matrix ([Synapse](https://github.com/matrix-org/synapse))
- [MegaLinter](https://github.com/oxsecurity/megalinter)
- Meltano ([Meltano CLI](https://github.com/meltano/meltano), [Singer SDK](https://github.com/meltano/sdk))
- Microsoft ([Semantic Kernel](https://github.com/microsoft/semantic-kernel),
    [ONNX Runtime](https://github.com/microsoft/onnxruntime),
    [LightGBM](https://github.com/microsoft/LightGBM))
- Modern Treasury ([Python SDK](https://github.com/Modern-Treasury/modern-treasury-python))
- Mozilla ([Firefox](https://github.com/mozilla/gecko-dev))
- [Mypy](https://github.com/python/mypy)
- [Nautobot](https://github.com/nautobot/nautobot)
- Netflix ([Dispatch](https://github.com/Netflix/dispatch))
- [Neon](https://github.com/neondatabase/neon)
- [Nokia](https://nokia.com/)
- [NoneBot](https://github.com/nonebot/nonebot2)
- [NumPyro](https://github.com/pyro-ppl/numpyro)
- [ONNX](https://github.com/onnx/onnx)
- [OpenBB](https://github.com/OpenBB-finance/OpenBBTerminal)
- [Open Wine Components](https://github.com/Open-Wine-Components/umu-launcher)
- [PDM](https://github.com/pdm-project/pdm)
- [PaddlePaddle](https://github.com/PaddlePaddle/Paddle)
- [Pandas](https://github.com/pandas-dev/pandas)
- [Pillow](https://github.com/python-pillow/Pillow)
- [Poetry](https://github.com/python-poetry/poetry)
- [Polars](https://github.com/pola-rs/polars)
- [PostHog](https://github.com/PostHog/posthog)
- Prefect ([Python SDK](https://github.com/PrefectHQ/prefect), [Marvin](https://github.com/PrefectHQ/marvin))
- [PyInstaller](https://github.com/pyinstaller/pyinstaller)
- [PyMC](https://github.com/pymc-devs/pymc/)
- [PyMC-Marketing](https://github.com/pymc-labs/pymc-marketing)
- [pytest](https://github.com/pytest-dev/pytest)
- [PyTorch](https://github.com/pytorch/pytorch)
- [Pydantic](https://github.com/pydantic/pydantic)
- [Pylint](https://github.com/PyCQA/pylint)
- [PyVista](https://github.com/pyvista/pyvista)
- [Reflex](https://github.com/reflex-dev/reflex)
- [River](https://github.com/online-ml/river)
- [Rippling](https://rippling.com)
- [Robyn](https://github.com/sansyrox/robyn)
- [Saleor](https://github.com/saleor/saleor)
- Scale AI ([Launch SDK](https://github.com/scaleapi/launch-python-client))
- [SciPy](https://github.com/scipy/scipy)
- Snowflake ([SnowCLI](https://github.com/Snowflake-Labs/snowcli))
- [Sphinx](https://github.com/sphinx-doc/sphinx)
- [Stable Baselines3](https://github.com/DLR-RM/stable-baselines3)
- [Starlette](https://github.com/encode/starlette)
- [Streamlit](https://github.com/streamlit/streamlit)
- [The Algorithms](https://github.com/TheAlgorithms/Python)
- [Vega-Altair](https://github.com/altair-viz/altair)
- WordPress ([Openverse](https://github.com/WordPress/openverse))
- [ZenML](https://github.com/zenml-io/zenml)
- [Zulip](https://github.com/zulip/zulip)
- [build (PyPA)](https://github.com/pypa/build)
- [cibuildwheel (PyPA)](https://github.com/pypa/cibuildwheel)
- [delta-rs](https://github.com/delta-io/delta-rs)
- [featuretools](https://github.com/alteryx/featuretools)
- [meson-python](https://github.com/mesonbuild/meson-python)
- [nox](https://github.com/wntrblm/nox)
- [pip](https://github.com/pypa/pip)

### Show Your Support

If you're using Ruff, consider adding the Ruff badge to your project's `README.md`:

```md
[![Ruff](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json)](https://github.com/astral-sh/ruff)
```

...or `README.rst`:

```rst
.. image:: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json
    :target: https://github.com/astral-sh/ruff
    :alt: Ruff
```

...or, as HTML:

```html
<a href="https://github.com/astral-sh/ruff"><img src="https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json" alt="Ruff" style="max-width:100%;"></a>
```

## License<a id="license"></a>

This repository is licensed under the [MIT License](https://github.com/astral-sh/ruff/blob/main/LICENSE)

<div align="center">
  <a target="_blank" href="https://astral.sh" style="background:none">
    <img src="https://raw.githubusercontent.com/astral-sh/ruff/main/assets/svg/Astral.svg" alt="Made by Astral">
  </a>
</div>
