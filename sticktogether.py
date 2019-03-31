import os, sys, math, glob

from PIL import Image

SCALE = 32 * 16

def get_coors(name):
    x = int(name.split(".")[1])
    y = int(name.split(".")[2])
    return x, y

path = "images"
names = glob.glob(path + "/*.png")

max_y, max_x = -math.inf, -math.inf
min_y, min_x = math.inf, math.inf
for name in names:
    x, y = get_coors(name)
    max_x = max(x, max_x)
    max_y = max(y, max_y)
    min_x = min(x, min_x)
    min_y = min(y, min_y)

width = (max_x - min_x) * SCALE
height = (max_y - min_y) * SCALE

complete = Image.new('RGB', (width, height))

for name in names:
    #img_path = os.path.join(path, name)
    img = Image.open(name)

    x, y = get_coors(name)
    x = (x - min_x) * SCALE
    y = (y - min_y) * SCALE

    complete.paste(img, (x, y))
    img.close()

complete.save('collage.png')
