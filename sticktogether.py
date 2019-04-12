import os, sys, math, glob

from PIL import Image

SCALE = 32 * 16

def get_coors(name):
    x = int(name.split(".")[-4])
    y = int(name.split(".")[-3])
    return x, y

def make_collage(folder):
    names = glob.glob(folder + "/*.png")

    max_y, max_x = -math.inf, -math.inf
    min_y, min_x = math.inf, math.inf
    for name in names:
        x, y = get_coors(name)
        print(x, y)
        max_x = max(x, max_x)
        max_y = max(y, max_y)
        min_x = min(x, min_x)
        min_y = min(y, min_y)

    width = (max_x - min_x + 1) * SCALE
    height = (max_y - min_y + 1) * SCALE

    print(width, height)

    complete = Image.new('RGB', (width, height))
    for name in names:
        img = Image.open(name)

        x, y = get_coors(name)
        x = (x - min_x) * SCALE
        y = (y - min_y) * SCALE

        complete.paste(img, (x, y))
        img.close()
    
    return complete

root = "images/*"
for folder in glob.glob(root):
    if os.path.isdir(folder):
        complete = make_collage(folder)
        complete.save(f'{folder}.png')