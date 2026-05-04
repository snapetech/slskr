"""
Setup configuration for soulseekR Python client
"""

from setuptools import setup, find_packages

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

setup(
    name="soulseekr-api-client",
    version="1.0.0",
    description="Official Python client for soulseekR HTTP API",
    long_description=long_description,
    long_description_content_type="text/markdown",
    author="soulseekR",
    url="https://github.com/soulseekr/soulseekr",
    packages=find_packages(),
    python_requires=">=3.7",
    install_requires=[
        "aiohttp>=3.8.0",
    ],
    extras_require={
        "dev": [
            "pytest>=6.0",
            "pytest-asyncio>=0.18.0",
            "black>=21.0",
            "flake8>=3.9",
            "mypy>=0.900",
        ],
    },
    classifiers=[
        "Development Status :: 5 - Production/Stable",
        "Intended Audience :: Developers",
        "Topic :: Software Development :: Libraries :: Python Modules",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.7",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
    ],
)
