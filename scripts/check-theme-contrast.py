#!/usr/bin/env python3
"""Hermetic WCAG 2.1 contrast gate over web/tailwind.css (stdlib only, no browser)."""

from __future__ import annotations

import math
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CSS = ROOT / "web" / "tailwind.css"
TEXT_MIN = 4.5  # Body/status text on its surface.
UI_MIN = 3.0  # Focus rings, rules, and control edges on adjacent surfaces.
# Decorative, non-essential pairs stay out of the gate: the quiet measurement
# grid, scrollbars, and slider tracks are never the only indicator of state.
TEXT_PAIRS = (
    ("foreground", "background"), ("card-foreground", "card"),
    ("popover-foreground", "popover"), ("primary-foreground", "primary"),
    ("secondary-foreground", "secondary"), ("muted-foreground", "muted"),
    ("muted-foreground", "background"), ("muted-foreground", "card"),
    ("foreground", "field"), ("accent-foreground", "accent"),
    ("destructive-foreground", "destructive"), ("success-foreground", "success"),
    ("warning-foreground", "warning"), ("info-foreground", "info"),
    ("selected-foreground", "selected"), ("selection-foreground", "selection"),
    ("keycap-ink", "keycap-face"), ("geometry-label", "geometry-body"),
    ("surface-foreground", "surface"),
)
UI_PAIRS = (
    ("ring", "background"), ("ring", "card"),
    ("border", "background"), ("border", "card"),
    ("border-strong", "background"), ("border-strong", "card"),
    ("input", "background"), ("input", "card"),
    ("primary", "background"), ("toolbar-border", "toolbar"),
    ("keycap-rule", "keycap-face"), ("geometry-board", "canvas"),
    ("geometry-key", "canvas"), ("geometry-part", "canvas"),
)
SHARED, BASE = ":root", ":root, [data-theme=original]"
ORIG_DARK, SHADCN, SHADCN_DARK = (
    ":root[data-appearance=dark]", "[data-theme=shadcn]",
    "[data-theme=shadcn][data-appearance=dark]")
# Cascade order per matrix; the original dark block outranks shadcn light, and
# the shadcn dark block (same specificity, later source) outranks both.
MATRICES = {
    "original x light": (SHARED, BASE),
    "original x dark": (SHARED, BASE, ORIG_DARK),
    "shadcn x light": (SHARED, BASE, SHADCN),
    "shadcn x dark": (SHARED, BASE, SHADCN, ORIG_DARK, SHADCN_DARK),
}


def _block(css: str, selector: str) -> dict[str, str]:
    pattern = ":root(?![\\[,])" if selector == SHARED else re.escape(selector)
    match = re.search(pattern + r"\s*\{([^}]*)\}", css)
    if not match:
        raise ValueError(f"theme block not found: {selector}")
    return {key: value.strip()
            for key, value in re.findall(r"(--[\w-]+)\s*:\s*([^;]+);", match.group(1))}


def _resolve(tokens: dict[str, str]) -> dict[str, str]:
    for _ in range(len(tokens)):
        done = True
        for key, value in tokens.items():
            nested = re.fullmatch(r"var\((--[\w-]+)\)", value)
            if nested and nested.group(1) in tokens:
                tokens[key] = tokens[nested.group(1)]
                done = False
        if done:
            return tokens
    raise ValueError("unresolvable var() alias in theme tokens")


def _to_linear(channel: float) -> float:
    return channel / 12.92 if channel <= 0.04045 else ((channel + 0.055) / 1.055) ** 2.4


def _oklch_to_srgb(lightness: float, chroma: float, hue_deg: float) -> tuple[float, float, float]:
    hue = math.radians(hue_deg)
    a, b = chroma * math.cos(hue), chroma * math.sin(hue)
    l = lightness + 0.3963377774 * a + 0.2158037573 * b
    m = lightness - 0.1055613458 * a - 0.0638541728 * b
    s = lightness - 0.0894841775 * a - 1.2914855480 * b
    l, m, s = l ** 3, m ** 3, s ** 3
    rgb = (4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
           -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
           -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s)

    def gamma(value: float) -> float:
        value = min(max(value, 0.0), 1.0)
        return 12.92 * value if value <= 0.0031308 else 1.055 * value ** (1 / 2.4) - 0.055

    return tuple(gamma(value) for value in rgb)


def _parse(color: str) -> tuple[float, float, float, float]:
    color = color.strip()
    if color == "transparent":
        return (0.0, 0.0, 0.0, 0.0)
    if match := re.fullmatch(r"#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})", color):
        digits = match.group(1)
        if len(digits) == 3:
            digits = "".join(char * 2 for char in digits)
        rgb = tuple(int(digits[i:i + 2], 16) / 255 for i in (0, 2, 4))
        return (*rgb, 1.0)
    if match := re.fullmatch(r"oklch\(([\d.]+) ([\d.]+) ([\d.]+)(?: / ([\d.]+%?))?\)", color):
        lightness, chroma, hue = (float(match.group(i)) for i in (1, 2, 3))
        alpha = match.group(4)
        alpha = float(alpha[:-1]) / 100 if alpha and alpha.endswith("%") else float(alpha or 1)
        return (*_oklch_to_srgb(lightness, chroma, hue), alpha)
    raise ValueError(f"unsupported theme color: {color}")


def _flatten_srgb(foreground: tuple[float, float, float, float],
                  background: tuple[float, float, float]) -> tuple[float, float, float]:
    alpha = foreground[3]
    return tuple(alpha * foreground[i] + (1.0 - alpha) * background[i] for i in range(3))


def _luminance(srgb: tuple[float, float, float]) -> float:
    red, green, blue = (_to_linear(channel) for channel in srgb)
    return 0.2126 * red + 0.7152 * green + 0.0722 * blue


def ratio(foreground: str, background: str, opaque_base: str) -> float:
    """WCAG contrast using CSS sRGB source-over compositing over opaque_base."""
    base = _parse(opaque_base)[:3]
    bg_rgba = _parse(background)
    back = _flatten_srgb(bg_rgba, base) if bg_rgba[3] < 1.0 else bg_rgba[:3]
    front = _flatten_srgb(_parse(foreground), back)
    light, dark = sorted((_luminance(front), _luminance(back)), reverse=True)
    return (light + 0.05) / (dark + 0.05)


def check_matrix(name: str, tokens: dict[str, str]) -> list[str]:
    failures = []
    for pairs, minimum in ((TEXT_PAIRS, TEXT_MIN), (UI_PAIRS, UI_MIN)):
        for fg_name, bg_name in pairs:
            value = ratio(tokens[f"--{fg_name}"], tokens[f"--{bg_name}"], tokens["--card"])
            if value < minimum:
                failures.append(f"{name}: {fg_name}/{bg_name} is {value:.2f}:1, needs >={minimum}:1")
    return failures


def main() -> int:
    css = CSS.read_text()
    blocks = {selector: _block(css, selector) for selector in (SHARED, BASE, ORIG_DARK, SHADCN, SHADCN_DARK)}
    failures, summary = [], []
    for name, selectors in MATRICES.items():
        merged = {}
        for selector in selectors:
            merged.update(blocks[selector])
        tokens = _resolve(merged)
        matrix_failures = check_matrix(name, tokens)
        failures.extend(matrix_failures)
        minima = []
        for pairs, minimum in ((TEXT_PAIRS, TEXT_MIN), (UI_PAIRS, UI_MIN)):
            worst = min((ratio(tokens[f"--{fg}"], tokens[f"--{bg}"], tokens["--card"]), fg, bg)
                        for fg, bg in pairs)
            minima.append(f"min {minimum}:1 pair {worst[1]}/{worst[2]} at {worst[0]:.2f}:1")
        status = "PASS" if not matrix_failures else f"FAIL ({len(matrix_failures)} pairs)"
        summary.append(f"{status} {name}: "
                       f"{len(TEXT_PAIRS)} text, {len(UI_PAIRS)} UI pairs; " + "; ".join(minima))
    print("\n".join([*failures, *summary]))
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
