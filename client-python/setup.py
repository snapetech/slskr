"""
Setup configuration for slskr Python client
"""

from setuptools import setup, find_packages

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

setup(
    name="slskr-api-client",
    version="1.0.0",
    description="Python client for the slskr HTTP API",
    long_description=long_description,
    long_description_content_type="text/markdown",
    author="slskr contributors",
    url="https://github.com/snapetech/slskr",
    packages=find_packages(),
    python_requires=">=3.10,<4",
    install_requires=[
        "aiohttp>=3.8.0,<4",
    ],
    extras_require={
        "dev": [
            "pytest>=8,<9",
            "pytest-asyncio>=0.23,<2",
            "black>=24,<26",
            "flake8>=7,<8",
            "mypy>=1.11,<2",
            "build>=1,<2",
        ],
    },
    classifiers=[
        "Development Status :: 5 - Production/Stable",
        "Intended Audience :: Developers",
        "Topic :: Software Development :: Libraries :: Python Modules",
        "License :: OSI Approved :: GNU Affero General Public License v3",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Programming Language :: Python :: 3.13",
    ],
)
