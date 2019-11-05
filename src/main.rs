mod loader;
mod map;
mod nbt;
mod renderer;

use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::RwLock;

fn folder_trim(path: &PathBuf) -> String {
    path.strip_prefix("/").unwrap_or(path).to_str().unwrap().replace('/', ":")
}

fn parse_name(name: &PathBuf) -> (i32, i32) {
    let sections: Vec<&str> = name.to_str().unwrap().split('.').collect();
    (sections[1].parse().unwrap(), sections[2].parse().unwrap())
}

// This should return the files list
fn save_images(files: &[(PathBuf, PathBuf)], generate_textures: bool) {
    // Load all the settings
    let ignore = loader::load_ignore_blocks().unwrap_or_else(|err| {
        println!("Error loading ignore blocks file: {}", err);
        std::process::exit(0)
    });
    let graphic_set = loader::load_graphic_props().unwrap_or_else(|err| {
        println!("Error loading blockstates from resources folder: {}", err);
        std::process::exit(0)
    });
    let biome_blocks = loader::load_biome_blocks().unwrap_or_else(|err| {
        println!("Error loading biome blocks file: {}", err);
        std::process::exit(0)
    });
    let textures = RwLock::new(loader::TextureLoader::new(biome_blocks));

    let progress = AtomicU32::new(0);
    // Generate all the images
    files.par_iter().for_each(|(region_path, image_path)| {
        let progress = progress.fetch_add(1, Ordering::SeqCst);

        let region_name = region_path.file_name().unwrap().to_str().unwrap();
        println!(
            "Generating new image for {} | {}/{} ({:.2}%)",
            region_name,
            progress + 1,
            files.len(),
            (progress + 1) as f32 / files.len() as f32 * 100.0
        );

        // If there was an error reading this region, generate an empty one
        let region = map::Region::from_file(&region_path, &graphic_set)
            .unwrap_or_else(|_| map::Region::new_empty());

        if generate_textures {
            renderer::image_chunk_textures(&region, &ignore, &textures).save(&image_path)
        } else {
            renderer::image_chunk(&region, &ignore, &textures).save(&image_path)
        }
        .unwrap();
    });
}

fn save_collage(
    images_folder: &PathBuf,
    files: &HashMap<(i32, i32), PathBuf>,
    resolution: (u32, u32),
) {
    let (xs, ys): (Vec<_>, Vec<_>) = files.keys().cloned().unzip();
    let min = (xs.iter().min().unwrap(), ys.iter().min().unwrap());
    let max = (xs.iter().max().unwrap(), ys.iter().max().unwrap());

    let scale = 32 * 16;

    let mut collage = image::RgbImage::new(
        (max.0 - min.0) as u32 * scale + 1,
        (max.1 - min.1) as u32 * scale + 1,
    );
    for (position, file) in files {
        let pixel = (
            (position.0 - min.0) as u32 * scale * resolution.0,
            (position.1 - min.1) as u32 * scale * resolution.1,
        );

        let img = image::open(file).unwrap();
        let img = img.as_rgb8().unwrap();
        image::imageops::replace(&mut collage, &img, pixel.0, pixel.1);
    }

    collage
        .save(images_folder.join("collage.png"))
        .unwrap();
}

fn main() {
    let yaml = clap::load_yaml!("cli.yml");
    let matches = clap::App::from_yaml(yaml).get_matches();

    // Get the command line arguments
    let generate_textures = matches.is_present("textures");
    let update = matches.is_present("update");
    let region_folder = Path::new(matches.value_of("world").unwrap()).join("region");

    println!("{}", folder_trim(&region_folder));
    // We move in the images_folder
    let images_folder = Path::new("images")
        .join(&folder_trim(&region_folder));
    
    println!("{}", images_folder.display());

    // Start the rendering
    std::fs::create_dir_all(&images_folder).unwrap_or_default();

    // Map files to their image_path
    let files: Vec<_> = fs::read_dir(region_folder)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .map(|region_path| {
            (
                region_path.clone(),
                // images_folder/name_of_region.png
                images_folder
                    .join(region_path.file_name().unwrap())
                    .with_extension("png"),
            )
        })
        .collect();

    // Get a list of all files that need updating
    let to_update = if update {
        files
            .iter()
            .filter(|(region_path, image_path)| {
                if let Ok(image_meta) = fs::metadata(&image_path) {
                    let region_meta = region_path.metadata().unwrap().modified().unwrap();
                    let image_meta = image_meta.modified().unwrap();

                    let region_time = region_meta
                        .duration_since(std::time::SystemTime::UNIX_EPOCH)
                        .unwrap();
                    let image_time = image_meta
                        .duration_since(std::time::SystemTime::UNIX_EPOCH)
                        .unwrap();

                    // Only keep this if the region was updated after the last rendering
                    region_time > image_time
                } else {
                    // The image wasn't even generated last rendering
                    true
                }
            })
            .cloned()
            .collect()
    } else {
        // Then all the files need to be updated
        files.clone()
    };

    // The files which need actual updating
    if !to_update.is_empty() {
        println!(
            "Only {} files need to be updated ({:.2}%)",
            to_update.len(),
            to_update.len() as f32 / files.len() as f32 * 100.0
        );
    } else {
        println!("Rendering up to date, no files need updating!");
    }

    // Generate the images which need to be updated
    save_images(&to_update, generate_textures);

    // The list of all generated regions with their coordinates attached
    let images: HashMap<(i32, i32), PathBuf> = files
        .iter()
        .map(|(_, image_path)| (parse_name(image_path), image_path.clone()))
        .collect();

    // Make a collage of images in which blocks are 16x16 pixels or 1x1 pixels
    println!("Generating collage image");
    if generate_textures {
        save_collage(&images_folder, &images, (16, 16));
    } else {
        save_collage(&images_folder, &images, (1, 1));
    }
}
