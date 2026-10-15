#!/usr/bin/env python3
"""Frame an app-only LinX screenshot for the project README.

The source image is preserved at native pixel size unless it is too large for
the requested canvas. This script packages screenshots only; it does not launch
or drive LinX, synthesize interface content, or capture the desktop.
"""

from __future__ import annotations

import argparse
from pathlib import Path

try:
    from PIL import Image, ImageDraw, ImageFilter, ImageFont
except ImportError:
    raise SystemExit("Screenshot helpers require Pillow (python3-pil on Debian 13).")


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_INPUT = ROOT / "local" / "current-control.png"
DEFAULT_OUTPUT = ROOT / "assets" / "screenshots" / "linx-console.png"
DEFAULT_CANVAS = (1200, 860)


def make_backdrop(size: tuple[int, int]) -> Image.Image:
    """Create a deep teal canvas with soft mint glows and a fine grid."""
    width, height = size
    base = Image.new("RGB", size, (7, 19, 21))

    glows = Image.new("RGBA", size, (0, 0, 0, 0))
    glow_draw = ImageDraw.Draw(glows)
    glow_draw.ellipse(
        (-width * 0.24, -height * 0.43, width * 0.65, height * 0.63),
        fill=(35, 201, 170, 48),
    )
    glow_draw.ellipse(
        (width * 0.42, height * 0.27, width * 1.25, height * 1.35),
        fill=(35, 173, 197, 43),
    )
    glow_draw.ellipse(
        (width * 0.25, height * 0.02, width * 0.86, height * 0.80),
        fill=(82, 224, 178, 18),
    )
    base = Image.alpha_composite(base.convert("RGBA"), glows.filter(ImageFilter.GaussianBlur(105)))

    grid = Image.new("RGBA", size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(grid)
    step = 40
    for x in range(0, width + 1, step):
        draw.line((x, 0, x, height), fill=(156, 238, 211, 13), width=1)
    for y in range(0, height + 1, step):
        draw.line((0, y, width, y), fill=(156, 238, 211, 13), width=1)

    # Gentle curved mesh lines add depth without competing with the UI.
    for offset in (-210, -70, 70, 210):
        points = []
        for step_index in range(81):
            t = step_index / 80
            x = t * width
            y = height * 0.52 + offset + (t - 0.5) ** 2 * height * 0.58
            points.append((x, y))
        draw.line(points, fill=(141, 236, 205, 17), width=1)
    return Image.alpha_composite(base, grid).convert("RGB")


def fit_without_upscaling(image: Image.Image, max_size: tuple[int, int]) -> Image.Image:
    scale = min(1.0, max_size[0] / image.width, max_size[1] / image.height)
    if scale == 1.0:
        return image
    fitted = (max(1, round(image.width * scale)), max(1, round(image.height * scale)))
    return image.resize(fitted, Image.Resampling.LANCZOS)


def frame(source: Image.Image, size: tuple[int, int], title: str | None) -> Image.Image:
    canvas = make_backdrop(size)
    margin_x, margin_y = 46, 54 if title else 42
    image = fit_without_upscaling(source.convert("RGB"), (size[0] - 2 * margin_x, size[1] - 2 * margin_y))
    x = (size[0] - image.width) // 2
    y = (size[1] - image.height) // 2 + (8 if title else 0)

    mask = Image.new("L", image.size, 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        (0, 0, image.width - 1, image.height - 1), radius=12, fill=255
    )
    shadow = Image.new("L", size, 0)
    shadow.paste(mask, (x, y + 16))
    shadow = shadow.filter(ImageFilter.GaussianBlur(25))
    shade = Image.new("RGB", size, (0, 0, 0))
    canvas = Image.composite(shade, canvas, shadow.point(lambda value: int(value * 0.62)))
    canvas.paste(image, (x, y), mask)
    ImageDraw.Draw(canvas).rounded_rectangle(
        (x, y, x + image.width - 1, y + image.height - 1),
        radius=12,
        outline=(78, 134, 119),
        width=1,
    )

    if title:
        draw = ImageDraw.Draw(canvas)
        font = ImageFont.load_default()
        draw.text((46, 25), title, fill=(197, 238, 219), font=font)
    return canvas


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, default=DEFAULT_INPUT, help="actual app-only PNG capture")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT, help="framed PNG destination")
    parser.add_argument("--width", type=int, default=DEFAULT_CANVAS[0], help="canvas width in pixels")
    parser.add_argument("--height", type=int, default=DEFAULT_CANVAS[1], help="canvas height in pixels")
    parser.add_argument("--title", help="optional small label above the screenshot")
    args = parser.parse_args()
    if not (320 <= args.width <= 6000 and 240 <= args.height <= 6000):
        parser.error("canvas dimensions must be between 320x240 and 6000x6000")
    if args.input.resolve() == args.output.resolve():
        parser.error("input and output must be different files")
    return args


def main() -> None:
    args = parse_args()
    if not args.input.is_file():
        raise SystemExit(f"Screenshot not found: {args.input}\nCapture the real LinX window first.")
    with Image.open(args.input) as opened:
        opened.load()
        result = frame(opened, (args.width, args.height), args.title)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    result.save(args.output, format="PNG", optimize=True)
    print(f"Saved {args.output} ({result.width}x{result.height})")


if __name__ == "__main__":
    main()
