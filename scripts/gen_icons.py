#!/usr/bin/env python3
"""Generate NP-Connect app icons in all required sizes."""

from PIL import Image, ImageDraw
import os, math

# ── Colours ──────────────────────────────────────────────────────────────────
INDIGO    = (99,  102, 241, 255)   # #6366f1 — main background
INDIGO_D  = (67,  70,  200, 255)   # #4346c8 — gradient shadow / inner shade
WHITE     = (238, 240, 246, 255)   # #eef0f6 — "NP" text
BG        = (11,  11,  24,  255)   # #0b0b18 — used only for tray disconnected

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
    draw.rectangle([x, y, x + lw - 1, y + h - 1], fill=color)
    draw.rectangle([x + w - lw, y, x + w - 1, y + h - 1], fill=color)
    pts = [
        (x + lw, y),
        (x + w - lw, y + h - 1),
        (x + w - 1, y + h - 1),
        (x + lw * 2, y),
    ]
    draw.polygon(pts, fill=color)

def draw_letter_P(draw, x, y, w, h, color, bg):
    """Draw a bold pixel 'P' in a bounding box (x,y,w,h)."""
    lw = max(2, w // 5)
    bump_h = h * 55 // 100
    bump_w = w * 70 // 100
    draw.rectangle([x, y, x + lw - 1, y + h - 1], fill=color)
    bx1, by1 = x + lw - 1, y
    bx2, by2 = x + bump_w - 1, y + bump_h - 1
    draw.ellipse([bx1, by1, bx2, by2], fill=color)
    inner_pad = lw
    ix1 = bx1 + lw
    iy1 = by1 + inner_pad
    ix2 = bx2 - inner_pad
    iy2 = by2 - inner_pad
    if ix2 > ix1 and iy2 > iy1:
        draw.ellipse([ix1, iy1, ix2, iy2], fill=bg)
    draw.rectangle([x, y, x + lw - 1, y + bump_h - 1], fill=color)

def make_icon(size):
    """Compose a single square icon — indigo bg + white NP."""
    img  = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    mask = rounded_rect_mask(size, radius_frac=0.22)

    # Indigo background
    bg_layer = Image.new("RGBA", (size, size), INDIGO)
    img.paste(bg_layer, mask=mask)

    # Subtle darker inner gradient — bottom quarter slightly deeper
    shade_h = size // 4
    shade = Image.new("RGBA", (size, shade_h), (*INDIGO_D[:3], 60))
    img.paste(shade, (0, size - shade_h), shade)

    # Re-apply rounded mask to clip the shade
    img.paste(bg_layer, mask=Image.eval(mask, lambda p: 0))  # clear outside
    img2 = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    img2.paste(img, mask=mask)
    img = img2

    # Draw "NP" letters in white
    draw = ImageDraw.Draw(img)
    letter_h  = int(size * 0.46)
    letter_w  = int(size * 0.24)
    gap       = int(size * 0.04)
    total_w   = letter_w * 2 + gap
    start_x   = (size - total_w) // 2
    start_y   = (size - letter_h) // 2

    draw_letter_N(draw, start_x, start_y, letter_w, letter_h, WHITE)
    draw_letter_P(draw, start_x + letter_w + gap, start_y, letter_w, letter_h, WHITE, INDIGO)

    return img

def make_tray(size, connected):
    """Small tray icon."""
    color = INDIGO if connected else (80, 83, 120, 200)
    img   = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    mask  = rounded_rect_mask(size, radius_frac=0.25)
    base  = Image.new("RGBA", (size, size), color)
    img.paste(base, mask=mask)
    draw  = ImageDraw.Draw(img)
    if size >= 32:
        lw = max(2, size // 8)
        h  = size * 6 // 10
        y0 = (size - h) // 2
        c  = WHITE
        draw.rectangle([size//5, y0, size//5 + lw - 1, y0 + h - 1], fill=c)
        draw.rectangle([size*2//5 - lw, y0, size*2//5 - 1, y0 + h - 1], fill=c)
        draw.rectangle([size*3//5, y0, size*3//5 + lw - 1, y0 + h - 1], fill=c)
    return img

# ── Output ────────────────────────────────────────────────────────────────────
OUT = os.path.join(os.path.dirname(__file__), "..", "src-tauri", "icons")

sizes = {
    "32x32.png":       32,
    "128x128.png":     128,
    "128x128@2x.png":  256,
    "icon_1024.png":   1024,
}

for fname, sz in sizes.items():
    path = os.path.join(OUT, fname)
    make_icon(sz).save(path)
    print(f"  {fname} ({sz}×{sz})")

make_tray(32, connected=True).save(os.path.join(OUT, "tray-connected.png"))
make_tray(32, connected=False).save(os.path.join(OUT, "tray-disconnected.png"))
print("  tray-connected.png + tray-disconnected.png")

print("Done. Run: npx @tauri-apps/cli icon src-tauri/icons/icon_1024.png -o src-tauri/icons")
