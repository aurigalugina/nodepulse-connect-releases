#!/usr/bin/env python3
"""Generate NP-Connect app icons in all required sizes."""

from PIL import Image, ImageDraw
import os, math

# ── Colours ──────────────────────────────────────────────────────────────────
BG        = (11,  11,  24,  255)   # #0b0b18 np-bg
CARD      = (18,  18,  31,  255)   # #12121f np-surface
BORDER    = (99,  102, 241, 180)   # #6366f1 indigo semi
INDIGO    = (99,  102, 241, 255)   # #6366f1
INDIGO_L  = (129, 140, 248, 255)   # #818cf8 np-indigo-light
WHITE     = (238, 240, 246, 255)   # #eef0f6 np-text

def rounded_rect_mask(size, radius_frac=0.22):
    """Return a mask image (L) with antialiased rounded corners."""
    s4 = size * 4
    r4 = int(s4 * radius_frac)
    mask4 = Image.new("L", (s4, s4), 0)
    d = ImageDraw.Draw(mask4)
    d.rounded_rectangle([0, 0, s4 - 1, s4 - 1], radius=r4, fill=255)
    return mask4.resize((size, size), Image.LANCZOS)

def draw_letter_N(draw, x, y, w, h, color):
    """Draw a bold pixel 'N' in a bounding box (x,y,w,h)."""
    lw = max(2, w // 5)
    # Left vertical
    draw.rectangle([x, y, x + lw - 1, y + h - 1], fill=color)
    # Right vertical
    draw.rectangle([x + w - lw, y, x + w - 1, y + h - 1], fill=color)
    # Diagonal: draw as filled polygon
    pts = [
        (x + lw, y),
        (x + w - lw, y + h - 1),
        (x + w - 1, y + h - 1),
        (x + lw * 2, y),
    ]
    draw.polygon(pts, fill=color)

def draw_letter_P(draw, x, y, w, h, color):
    """Draw a bold pixel 'P' in a bounding box (x,y,w,h)."""
    lw = max(2, w // 5)
    bump_h = h * 55 // 100          # top 55% has the bump
    bump_w = w * 70 // 100
    bump_r = bump_h // 2
    # Vertical stem
    draw.rectangle([x, y, x + lw - 1, y + h - 1], fill=color)
    # Upper bump (filled arc via ellipse)
    bx1, by1 = x + lw - 1, y
    bx2, by2 = x + bump_w - 1, y + bump_h - 1
    draw.ellipse([bx1, by1, bx2, by2], fill=color)
    # Knock out interior of bump
    inner_pad = lw
    ix1 = bx1 + lw
    iy1 = by1 + inner_pad
    ix2 = bx2 - inner_pad
    iy2 = by2 - inner_pad
    if ix2 > ix1 and iy2 > iy1:
        draw.ellipse([ix1, iy1, ix2, iy2], fill=CARD)
    # Plug the left side of the ellipse cutout so stem stays solid
    draw.rectangle([x, y, x + lw - 1, y + bump_h - 1], fill=color)

def make_icon(size):
    """Compose a single square icon of given pixel size."""
    pad  = max(4,  int(size * 0.06))
    inner_pad = max(2, int(size * 0.03))

    # Base transparent canvas
    img  = Image.new("RGBA", (size, size), (0, 0, 0, 0))

    # Outer rounded square — indigo glow border
    border_mask = rounded_rect_mask(size, radius_frac=0.22)
    bg_layer = Image.new("RGBA", (size, size), BORDER)
    img.paste(bg_layer, mask=border_mask)

    # Inner card — dark background
    inner_size = size - pad * 2
    inner_img  = Image.new("RGBA", (inner_size, inner_size), (0, 0, 0, 0))
    inner_mask = rounded_rect_mask(inner_size, radius_frac=0.18)
    card_layer = Image.new("RGBA", (inner_size, inner_size), CARD)
    inner_img.paste(card_layer, mask=inner_mask)
    img.paste(inner_img, (pad, pad), inner_img)

    # Draw "NP" letters
    draw = ImageDraw.Draw(img)

    letter_h  = int(size * 0.44)
    letter_w  = int(size * 0.22)
    gap       = int(size * 0.04)
    total_w   = letter_w * 2 + gap
    start_x   = (size - total_w) // 2
    start_y   = (size - letter_h) // 2

    draw_letter_N(draw, start_x, start_y, letter_w, letter_h, INDIGO_L)
    draw_letter_P(draw, start_x + letter_w + gap, start_y, letter_w, letter_h, INDIGO_L)

    return img

def make_tray(size, connected):
    """Small tray icon."""
    color = INDIGO if connected else (80, 83, 120, 200)
    img   = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    mask  = rounded_rect_mask(size, radius_frac=0.25)
    base  = Image.new("RGBA", (size, size), color)
    img.paste(base, mask=mask)
    draw  = ImageDraw.Draw(img)
    # Simple "NP" hint — just two thin vertical bars at tiny sizes
    if size >= 32:
        lw = max(2, size // 8)
        h  = size * 6 // 10
        y0 = (size - h) // 2
        c  = WHITE
        # N stems
        draw.rectangle([size//5, y0, size//5 + lw - 1, y0 + h - 1], fill=c)
        draw.rectangle([size*2//5 - lw, y0, size*2//5 - 1, y0 + h - 1], fill=c)
        # P stem
        draw.rectangle([size*3//5, y0, size*3//5 + lw - 1, y0 + h - 1], fill=c)
    return img

# ── Output ────────────────────────────────────────────────────────────────────
OUT = os.path.join(os.path.dirname(__file__), "..", "src-tauri", "icons")

sizes = {
    "32x32.png":       32,
    "128x128.png":     128,
    "128x128@2x.png":  256,
    "icon_1024.png":   1024,   # source for Tauri icon generator
}

for fname, sz in sizes.items():
    path = os.path.join(OUT, fname)
    make_icon(sz).save(path)
    print(f"  {fname} ({sz}×{sz})")

# Tray icons
make_tray(32, connected=True).save(os.path.join(OUT, "tray-connected.png"))
make_tray(32, connected=False).save(os.path.join(OUT, "tray-disconnected.png"))
print("  tray-connected.png + tray-disconnected.png")

print("Done. Run: npx @tauri-apps/cli icon src-tauri/icons/icon_1024.png -o src-tauri/icons")
