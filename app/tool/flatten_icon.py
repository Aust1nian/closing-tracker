"""Writes the opaque app icons Apple and Android accept from render_icon.mjs's PNG.

usage: python3 tool/flatten_icon.py <rendered.png>   (run from app/)
"""

import sys
from pathlib import Path

from PIL import Image

src = Image.open(sys.argv[1]).convert("RGBA")
white = Image.new("RGBA", src.size, (255, 255, 255, 255))
flat = Image.alpha_composite(white, src).convert("RGB")  # no alpha channel: App Store rule

ios = Path("ios/Runner/Assets.xcassets/AppIcon.appiconset")
for old in ios.glob("*.png"):
    old.unlink()
flat.resize((1024, 1024), Image.LANCZOS).save(ios / "AppIcon-1024.png", optimize=True)
(ios / "Contents.json").write_text(
    """{
  "images" : [
    {
      "filename" : "AppIcon-1024.png",
      "idiom" : "universal",
      "platform" : "ios",
      "size" : "1024x1024"
    }
  ],
  "info" : {
    "author" : "xcode",
    "version" : 1
  }
}
"""
)

android = {"mdpi": 48, "hdpi": 72, "xhdpi": 96, "xxhdpi": 144, "xxxhdpi": 192}
for density, px in android.items():
    flat.resize((px, px), Image.LANCZOS).save(
        Path(f"android/app/src/main/res/mipmap-{density}/ic_launcher.png"), optimize=True
    )
print("icons written")
