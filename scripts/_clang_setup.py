"""Configure libclang. Imported (not run) by other scripts."""

from __future__ import annotations

import os
from pathlib import Path

import clang.cindex

# macOS: Homebrew LLVM. Override with LIBCLANG_PATH env if needed.
DEFAULT_LIBCLANG = "/opt/homebrew/opt/llvm/lib/libclang.dylib"


def configure() -> None:
    # Only honor LIBCLANG_PATH if it points to an actual dylib file.
    # The user has it set to a directory (ESP toolchain), which would
    # confuse libclang's loader.
    env_path = os.environ.get("LIBCLANG_PATH", "")
    if env_path and Path(env_path).is_file():
        path = env_path
    else:
        path = DEFAULT_LIBCLANG
    if not Path(path).is_file():
        raise RuntimeError(
            f"libclang not found at {path}. "
            f"Set LIBCLANG_PATH to a libclang.dylib, or install llvm via brew."
        )
    clang.cindex.Config.set_library_file(path)


configure()
