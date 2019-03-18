use serde_json::Value;
use std::fs;

mod loader;
mod map;
mod nbt;

use std::collections::{HashMap, HashSet};
use image::ImageBuffer;

fn image_chunk(
    region: &map::Region,
    ignore: &HashSet<String>,
    textures: &mut loader::TextureLoader,
    file_name: &str,
) {

    ImageBuffer::from_fn(16 * 32, 16 * 32, |x_block, z_block| {
        let (x_block, z_block) = (x_block as usize, z_block as usize);
        for y in (0..256).rev() {
            let candidate = region.get_block(x_block, y, z_block);
            if !ignore.contains(candidate.as_str()) {
                let properties = region.get_gprop(x_block, y, z_block);
                if let Some(index) = textures.load(candidate, properties) {
                    let (_, _, avg) = textures.get_texture(index);
                    return image::Rgb(*avg);
                }
            }
        }
        image::Rgb([0, 0, 0])
    })
    .save(file_name)
    .unwrap();
}


fn overlay(bottom: &mut image::RgbaImage, top: &image::RgbaImage, x: u32, y: u32) {
    for dx in 0..top.width() {
        for dy in 0..top.height() {
            let top_pixel = top.get_pixel(dx, dy);
            let bottom_pixel =
                bottom.get_pixel(x + dx, y + dy);

            // Only paint if this pixel is invisible
            if bottom_pixel[3] == 0 {
                bottom.put_pixel(
                    x + dx,
                    y + dy,
                    *top_pixel,
                );
            }
        }
    }
}

fn image_chunk_textures(
    region: &map::Region,
    ignore: &HashSet<String>,
    textures: &mut loader::TextureLoader,
    file_name: &str,
) {

    let mut img: image::RgbaImage = ImageBuffer::new(16 * 32 * 16, 16 * 32 * 16);
    for x in 0..(16 * 32) {
        for z in 0..(16 * 32) {
            for y in (0..256).rev() {
                let candidate = region.get_block(x, y, z);
                if !ignore.contains(&candidate) {
                    let properties = region.get_gprop(x, y, z);
                    if let Some(index) = textures.load(candidate, properties) {
                        let (texture, is_trasparent, _) = textures.get_texture(index);
                        overlay(&mut img, &texture, x as u32 * 16, z as u32 * 16);

                        // If this block is trasparent find the lower blocks
                        if !is_trasparent {
                            break;
                        }
                    }
                }
            }
        }
    }
    img.save(file_name).unwrap();
}

fn get_graphic_set() -> HashMap<String, HashMap<String, usize>> {
    let mut graphic_set = HashMap::new();
    for entry in fs::read_dir("resources/blockstates").unwrap() {
        let entry = entry.unwrap();

        // Append the minecraft namespace
        let path = entry.path();
        let name = format!("minecraft:{}", path.file_stem().unwrap().to_str().unwrap());

        let text = fs::read_to_string(path).unwrap();
        let json: Value = serde_json::from_str(&text).unwrap();

        let mut used_variants = HashMap::new();
        if let Some(variants) = json.get("variants") {
            let variants = variants.as_object().unwrap();
            for variant in variants.keys() {
                let keyvalues = variant.split(',');
                for (i, keyvalue) in keyvalues.enumerate() {
                    let key = keyvalue.split('=').nth(0);
                    if let Some(key) = key {
                        if key != "" {
                            used_variants.insert(key.to_string(), i);
                        }
                    }
                }
            }
        }

        graphic_set.insert(name, used_variants);
    }

    graphic_set
}

fn get_ignore_set() -> HashSet<String> {
    let ignore_json = fs::read_to_string("settings/ignore_blocks.json").unwrap();
    let ignore_json: Vec<Value> = serde_json::from_str(&ignore_json).unwrap();

    let mut ignore = HashSet::new();
    for block in &ignore_json {
        let block_name = block.as_str().unwrap();
        ignore.insert(block_name.to_string());
    }

    ignore
}

fn main() {
    let ignore = get_ignore_set();
    let graphic_set = get_graphic_set();

    let mut textures = loader::TextureLoader::new();
    for entry in fs::read_dir("region").unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().into_string().unwrap();

        println!("{}", name);
        let region = map::Region::from_file(entry.path().to_str().unwrap(), &graphic_set);
        image_chunk_textures(
            &region,
            &ignore,
            &mut textures,
            &format!("images/{}.png", name),
        );
    }
}
