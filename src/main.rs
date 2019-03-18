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
                    let (_, avg) = textures.get_texture(index);
                    return image::Rgb(*avg);
                }
            }
        }
        image::Rgb([0, 0, 0])
    })
    .save(file_name)
    .unwrap();
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
            'down: for y in (0..256).rev() {
                let candidate = region.get_block(x, y, z);
                if !ignore.contains(&candidate) {
                    let properties = region.get_gprop(x, y, z);
                    if let Some(index) = textures.load(candidate, properties) {
                        let (texture, _) = textures.get_texture(index);
                        let mut transparent = false;
                        for tx in 0..16 {
                            for tz in 0..16 {
                                let mut target_pixel = *texture.get_pixel(tx, tz);
                                let original_pixel =
                                    *img.get_pixel(x as u32 * 16 + tx, z as u32 * 16 + tz);

                                if target_pixel[3] == 0 {
                                    transparent = true;
                                } else if target_pixel[3] != 255 {
                                    target_pixel[3] = 255;
                                }

                                // Only paint if this pixel is semi invisible
                                if original_pixel[3] == 0 {
                                    img.put_pixel(
                                        x as u32 * 16 + tx,
                                        z as u32 * 16 + tz,
                                        target_pixel,
                                    );
                                }
                            }
                        }

                        if !transparent {
                            break 'down;
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
        image_chunk(
            &region,
            &ignore,
            &mut textures,
            &format!("images/{}.png", name),
        );
    }
}
