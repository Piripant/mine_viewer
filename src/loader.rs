use serde_json::Value;
use std::fs;

use std::collections::{HashMap, HashSet};

fn image_avg(img: &image::RgbaImage) -> [u8; 3] {
    let mut r = 0;
    let mut g = 0;
    let mut b = 0;
    let mut n = 1;
    for pixel in img.pixels() {
        if pixel[3] != 0 {
            r += u32::from(pixel[0]);
            g += u32::from(pixel[1]);
            b += u32::from(pixel[2]);
            n += 1;
        }
    }

    [(r / n) as u8, (g / n) as u8, (b / n) as u8]
}

fn clamp(value: i16, min: i16, max: i16) -> i16 {
    if value > max {
        max
    } else if value < min {
        min
    } else {
        value
    }
}

fn taint_image(img: &mut image::RgbaImage, taint: [i16; 3]) {
    for pixel in img.pixels_mut() {
        for i in 0..3 {
            pixel[i] = clamp(i16::from(pixel[i]) + taint[i], 0, 255) as u8;
        }
    }
}

fn get_texture(model: &str) -> Option<String> {
    let path = format!("resources/models/{}.json", model);
    let text = fs::read_to_string(path).unwrap();
    let json: Value = serde_json::from_str(&text).unwrap();

    // top, all, or particle are the textures
    // more often visualized from top
    if let Some(textures) = json.get("textures") {
        if let Some(top) = textures.get("top") {
            Some(top.as_str().unwrap().to_owned())
        } else if let Some(all) = textures.get("all") {
            Some(all.as_str().unwrap().to_owned())
        } else if let Some(particle) = textures.get("particle") {
            Some(particle.as_str().unwrap().to_owned())
        } else {
            None
        }
    } else {
        None
    }
}

fn get_model(name: &str, properties: &str) -> Option<String> {
    let path = format!("resources/blockstates/{}.json", &name[10..]);
    let text = fs::read_to_string(path).unwrap();
    let json: Value = serde_json::from_str(&text).unwrap();

    if let Some(variants) = json.get("variants") {
        if let Some(variant) = variants.get(properties) {
            // Some blocks have different models for the same variant
            // Which in the game are choosen at random
            // We always choose the first one to have a better performance
            if variant.is_array() {
                Some(variant[0]["model"].as_str().unwrap().to_owned())
            } else {
                Some(variant["model"].as_str().unwrap().to_owned())
            }
        } else {
            panic!("Couldn't find model of block {} with {} properties", name, properties);
        }
    } else {
        None
    }
}

fn is_transparent(img: &image::RgbaImage) -> bool {
    for pixel in img.pixels() {
        if pixel[3] != 255 {
            return true;
        }
    }
    false
}

pub struct TextureLoader {
    // Vec<texture, is_trasparent, average color>
    textures: Vec<(image::RgbaImage, bool, [u8; 3])>,
    // HashMap<(block name, block properties), Option<texture index>>
    textures_map: HashMap<(String, String), Option<usize>>,
    // Block which have a white and gray texture that needs to be painted
    biome_blocks: HashMap<String, [i16; 3]>,
}

impl TextureLoader {
    pub fn new(biome_blocks: HashMap<String, [i16; 3]>) -> TextureLoader {
        TextureLoader {
            textures: Vec::new(),
            textures_map: HashMap::new(),
            biome_blocks,
        }
    }

    pub fn get_texture(&self, index: usize) -> &(image::RgbaImage, bool, [u8; 3]) {
        &self.textures[index]
    }

    pub fn load(&mut self, name: &str, properties: &str) -> Option<usize> {
        // Check if the texture was already loaded
        if let Some(index) = self
            .textures_map
            .get(&(name.to_owned(), properties.to_owned()))
        {
            return *index;
        }

        // Try to load the texture
        if let Some(model) = get_model(name, properties) {
            if let Some(texture) = get_texture(&model) {
                let texture = format!("resources/textures/{}.png", texture);

                // Crop the texture in case it is a texture strip for an animated block
                let mut texture = image::open(texture).unwrap().crop(0, 0, 16, 16).to_rgba();

                // The color must be tainted for blocks like leaves, grass and water
                // Sometimes the taint is hardcoded in minecraft
                // so the only way to reproduce is to define it ourselves
                if let Some(taint) = self.biome_blocks.get(name) {
                    taint_image(&mut texture, *taint);
                }
                let avg = image_avg(&texture);

                let is_transparent = is_transparent(&texture);
                self.textures.push((texture, is_transparent, avg));
                self.textures_map.insert(
                    (name.to_owned(), properties.to_owned()),
                    Some(self.textures.len() - 1),
                );
                return Some(self.textures.len() - 1);
            }
        }

        // If we get to this point we where unable to load the texture
        // So we flag it as unloadable for the future
        self.textures_map
            .insert((name.to_owned(), properties.to_owned()), None);
        None
    }
}

const SETTINGS_FOLDER: &str = "settings/";
const BIOME_BLOCKS_FILE: &str = "biome_blocks.json";
const IGNORE_BLOCKS_FILE: &str = "ignore_blocks.json";

const BLOCKSTATES_FOLDER: &str = "resources/blockstates";

use std::io;

pub fn load_ignore_blocks() -> io::Result<HashSet<String>> {
    let ignore_json = fs::read_to_string(SETTINGS_FOLDER.to_owned() + IGNORE_BLOCKS_FILE)?;
    Ok(serde_json::from_str(&ignore_json)?)
}

pub fn load_biome_blocks() -> io::Result<HashMap<String, [i16; 3]>> {
    let biome_blocks = fs::read_to_string(SETTINGS_FOLDER.to_owned() + BIOME_BLOCKS_FILE)?;
    Ok(serde_json::from_str(&biome_blocks)?)
}

pub fn load_graphic_props() -> io::Result<HashMap<String, HashMap<String, usize>>> {
    let mut graphic_set = HashMap::new();
    for entry in fs::read_dir(BLOCKSTATES_FOLDER)? {
        let entry = entry?;

        // Check if the file has a valid name and append the minecraft namespace
        let path = entry.path();
        let name = if let Some(name) = path.file_stem() {
            format!("minecraft:{}", name.to_str().unwrap())
        } else {
            continue;
        };

        let text = fs::read_to_string(path)?;
        let json: Value = serde_json::from_str(&text)?;

        let mut used_variants = HashMap::new();
        if let Some(variants) = json.get("variants") {
            let variants: serde_json::Map<String, Value> = serde_json::from_value(variants.clone())?;

            // The keys are the graphical properties used for each variant
            // in the format `prop1=value1,prop2=value,prop3=... etc`
            // take a look at acacia_door.json in blockstates/ to understand better
            for variant in variants.keys() {
                // Separate each property-value pair
                let propvalues = variant.split(',');
                for (i, prop_value) in propvalues.enumerate() {
                    // Get the property
                    if let Some(prop) = prop_value.split('=').nth(0) {
                        if prop != "" {
                            used_variants.insert(prop.to_owned(), i);
                        }
                    }
                }
            }
        }

        graphic_set.insert(name, used_variants);
    }

    Ok(graphic_set)
}