use std::fs;

mod loader;
mod map;
mod nbt;

use image::ImageBuffer;
use std::collections::HashSet;

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
            if !ignore.contains(candidate) {
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
            let bottom_pixel = bottom.get_pixel(x + dx, y + dy);

            // Only paint if this pixel is invisible
            if bottom_pixel[3] == 0 {
                bottom.put_pixel(x + dx, y + dy, *top_pixel);
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
                if !ignore.contains(candidate) {
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

use clap::{App, Arg};
fn main() {
    let matches = App::new("mineviewer")
        .author("Piripant")
        .about("Renders a top view of a minecraft world to a png file")
        .arg(
            Arg::with_name("region")
                .short("r")
                .long("region")
                .value_name("REGION_FOLDER")
                .help("Sets a custom region folder")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("textures")
                .short("t")
                .long("textures")
                .help("Sets if the generated image is composed of textures (rather than single pixels)")
        )
        .arg(
            Arg::with_name("update")
                .short("u")
                .long("update")
                .help("Only renders regions that have been updated since the last rendering (might not render some updated regions)")
        )
        .get_matches();

    let ignore = loader::load_ignore_blocks().unwrap_or_else(|err| {
        println!("Error loading ignore blocks file: {}", err);
        std::process::exit(0)
    });
    let graphic_set = loader::load_graphic_props().unwrap_or_else(|err| {
        println!("Error loading files in blockstates: {}", err);
        std::process::exit(0)
    });

    let generate_textures = matches.is_present("textures");
    let check_time = matches.is_present("update");

    let region_folder = matches.value_of("region").unwrap_or("region");

    let mut textures =
        loader::TextureLoader::new(loader::load_biome_blocks().unwrap_or_else(|err| {
            println!("Error loading biome blocks file: {}", err);
            std::process::exit(0)
        }));

    std::fs::create_dir("images").unwrap_or_default();

    let files: Vec<std::fs::DirEntry> = fs::read_dir(region_folder)
        .unwrap()
        .map(|entry| entry.unwrap())
        .collect();

    for (i, entry) in files.iter().enumerate() {
        let region_name = entry.file_name().into_string().unwrap();

        let image_name = format!("images/{}.png", region_name);

        let generate = if !check_time {
            true
        } else if let Ok(image_meta) = fs::metadata(&image_name) {
            let region_meta = entry.metadata().unwrap().modified().unwrap();
            let image_meta = image_meta.modified().unwrap();

            let region_time = region_meta
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap();
            let image_time = image_meta
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap();
            region_time > image_time
        } else {
            true
        };

        if generate {
            print!("Generating new image for {}", region_name);
            // If there was an error reading this region, generate an empty one
            let region = map::Region::from_file(entry.path().to_str().unwrap(), &graphic_set)
                .unwrap_or_else(|_| map::Region::new_empty());

            if generate_textures {
                image_chunk_textures(&region, &ignore, &mut textures, &image_name);
            } else {
                image_chunk(&region, &ignore, &mut textures, &image_name);
            }
        } else {
            print!("Skipping {}, nothing new", region_name);
        }

        println!(" {}/{}", i + 1, files.len());
    }
}
