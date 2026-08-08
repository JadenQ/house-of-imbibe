"""Generate a synthetic test image for image2pixel demo.

A stylized "red panda" face on a solid background — gives the vision model
clear visual features (color, shape, distinctive markings) to describe.
"""
from PIL import Image, ImageDraw

W = H = 512
img = Image.new("RGB", (W, H), (135, 180, 220))  # sky blue background
d = ImageDraw.Draw(img)

# Body (rounded rectangle, warm rust color)
body_color = (180, 70, 50)      # rust red
belly_color = (90, 40, 30)      # dark belly
face_color = (220, 180, 150)    # pale face
ear_color = (60, 30, 20)        # dark ears

# Body
d.ellipse([120, 200, 392, 480], fill=body_color)
# Belly patch
d.ellipse([200, 320, 312, 470], fill=belly_color)

# Head
d.ellipse([150, 100, 362, 290], fill=body_color)
# Face mask
d.ellipse([185, 150, 327, 280], fill=face_color)
# White muzzle
d.ellipse([220, 215, 292, 260], fill=(245, 230, 220))
# Nose
d.ellipse([248, 222, 264, 235], fill=(30, 20, 20))
# Mouth
d.arc([238, 230, 274, 250], start=0, end=180, fill=(30, 20, 20), width=2)

# Eyes
for cx in (215, 297):
    d.ellipse([cx-12, 175, cx+12, 200], fill=(40, 30, 25))
    d.ellipse([cx-5, 182, cx+5, 192], fill=(255, 255, 255))
    d.ellipse([cx-3, 184, cx+3, 190], fill=(10, 10, 10))

# Ears
d.ellipse([155, 90, 215, 155], fill=ear_color)
d.ellipse([297, 90, 357, 155], fill=ear_color)
# Inner ear
d.ellipse([168, 105, 202, 145], fill=body_color)
d.ellipse([310, 105, 344, 145], fill=body_color)

# Tail (curled, striped)
d.arc([330, 280, 460, 410], start=20, end=270, fill=body_color, width=42)
for i, off in enumerate([0, 30, 60, 90]):
    d.arc([330+off//3, 280+off//3, 460-off//3, 410-off//3],
          start=20, end=270, fill=ear_color, width=6)

img.save("pixellab-out/test-red-panda.png", "PNG", optimize=True)
print("Wrote pixellab-out/test-red-panda.png", img.size, img.mode)