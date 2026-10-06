"""Shared KiCad form reader for source-layout maintenance commands.

Forms are nested lists. Atoms are strings; quoted strings carry a leading NUL
so they stay distinguishable from atoms until `value` unwraps them.
"""

from __future__ import annotations

import json
import re

TOKEN = re.compile(r'\s*(\(|\)|"(?:\\.|[^"\\])*"|[^\s()]+)')
QUOTED = "\0"


def _tokens(source: str) -> list[str]:
    result = []
    offset = 0
    while offset < len(source):
        match = TOKEN.match(source, offset)
        if not match:
            if source[offset:].strip() == "":
                break
            raise ValueError(f"Invalid KiCad source at byte {offset}")
        result.append(match.group(1))
        offset = match.end()
    return result


def parse_forms(source: str) -> list[list]:
    tokens = _tokens(source)
    cursor = 0

    def read():
        nonlocal cursor
        if cursor >= len(tokens) or tokens[cursor] == ")":
            raise ValueError("Unbalanced KiCad source")
        token = tokens[cursor]
        cursor += 1
        if token != "(":
            return QUOTED + json.loads(token) if token.startswith('"') else token
        result = []
        while True:
            if cursor >= len(tokens):
                raise ValueError("Unbalanced KiCad source")
            if tokens[cursor] == ")":
                break
            result.append(read())
        cursor += 1
        return result

    forms = []
    while cursor < len(tokens):
        node = read()
        if not isinstance(node, list):
            raise ValueError("KiCad source must contain KiCad forms")
        forms.append(node)
    return forms


def value(node) -> str:
    if not isinstance(node, str):
        raise ValueError("Expected KiCad scalar")
    return node[1:] if node.startswith(QUOTED) else node


def child(node: list, name: str):
    return next((item for item in node if isinstance(item, list) and item and item[0] == name), None)
