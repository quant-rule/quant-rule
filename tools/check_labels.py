#!/usr/bin/env python3
"""Fail when the generated client label table drifts from the registry."""

from __future__ import annotations

import sys

from generate_labels import check

if __name__ == "__main__":
    sys.exit(check())
