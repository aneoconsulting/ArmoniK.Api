
# Configuration file for the Sphinx documentation builder.
#
# For the full list of built-in configuration values, see the documentation:
# https://www.sphinx-doc.org/en/master/usage/configuration.html

# -- Project information -----------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#project-information

project = "ArmoniK.Api"
copyright = "2021-%Y, ANEO"
author = "ANEO"
release = "main"

import sys
import os
sys.path.insert(0, os.path.abspath('../packages/python/src'))

# -- General configuration ---------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#general-configuration

extensions = ["myst_parser",
    'sphinxcontrib.mermaid',
    "sphinx.ext.autosummary",
    "sphinx.ext.napoleon",
    "sphinx.ext.autodoc",
]

templates_path = ["_templates"]
exclude_patterns = ["requirements.txt", "README.md"]
suppress_warnings = ["myst.header"]

# -- Options for HTML output -------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#options-for-html-output

html_theme = "sphinx_rtd_theme"
html_static_path = ["_static"]
html_search = True

# -- Options for source files ------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#options-for-source-files
source_suffix = {
    ".rst": "restructuredtext",
    ".txt": "markdown",
    ".md": "markdown",
}

autodoc_mock_imports = [
    'armonik._version', 
    "grpc", 
    "cryptography",
    "armonik.protogen"
]

# -- Options MyST Parser ------------------------------------------------
myst_fence_as_directive = ["mermaid"]
myst_heading_anchors = 3

# -- Options to show "Edit on GitHub" button ---------------------------------
html_context = {
    "display_github": True, # Integrate GitHub
    "github_user": "aneoconsulting", # Username
    "github_repo": "ArmoniK.Api", # Repo name
    "github_version": "main", # Version
    "conf_py_path": "/.docs/", # Path in the checkout to the docs root
}

