mod loader;
mod map;
mod nbt;
mod renderer;

use std::collections::HashMap;
use std::fs;
use std::fs::DirEntry;

fn folder_name(path: &str) -> String {
    path.replace('/', ":")
}

fn parse_name(name: &str) -> (i32, i32) {
    let sections: Vec<&str> = name.split('.').collect();
    (sections[1].parse().unwrap(), sections[2].parse().unwrap())
}

fn save_images(files: Vec<DirEntry>, images_folder: &str, generate_textures: bool) {
    // Load all the settings
    let ignore = loader::load_ignore_blocks().unwrap_or_else(|err| {
        println!("Error loading ignore blocks file: {}", err);
        std::process::exit(0)
    });
    let graphic_set = loader::load_graphic_props().unwrap_or_else(|err| {
        println!("Error loading blockstates from resources folder: {}", err);
        std::process::exit(0)
    });
    let mut textures =
        loader::TextureLoader::new(loader::load_biome_blocks().unwrap_or_else(|err| {
            println!("Error loading biome blocks file: {}", err);
            std::process::exit(0)
        }));

    // Generate all the images
    for (i, entry) in files.iter().enumerate() {
        let region_name = entry.file_name().into_string().unwrap();
        let image_name = format!("{}/{}.png", images_folder, region_name);

        println!(
            "Generating new image for {} | {}/{} ({:.2}%)",
            region_name,
            i + 1,
            files.len(),
            (i + 1) as f32 / files.len() as f32 * 100.0
        );

        // If there was an error reading this region, generate an empty one
        let region = map::Region::from_file(&entry.path(), &graphic_set)
            .unwrap_or_else(|_| map::Region::new_empty());

        if generate_textures {
            renderer::image_chunk_textures(&region, &ignore, &mut textures).save(&image_name)
        } else {
            renderer::image_chunk(&region, &ignore, &mut textures).save(&image_name)
        }
        .unwrap();
    }
}

fn save_collage(
    images_folder: &str,
    files: &HashMap<(i32, i32), String>,
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
        .save(format!("{}/collage.png", images_folder))
        .unwrap();
}

fn main() {
    let yaml = clap::load_yaml!("cli.yml");
    let matches = clap::App::from_yaml(yaml).get_matches();

    // Get the command line arguments
    let generate_textures = matches.is_present("textures");
    let update = matches.is_present("update");
    let region_folder = matches.value_of("world").unwrap().to_owned() + "/region";

    let images_folder = format!("images/{}", folder_name(&region_folder));

    // Start the rendering
    std::fs::create_dir_all(&images_folder).unwrap_or_default();

    let mut files: Vec<_> = fs::read_dir(region_folder)
        .unwrap()
        .map(|entry| entry.unwrap())
        .collect();

    let mut images = HashMap::new();
    // Get a list of all files that need updating
    if update {
        let original_len = files.len();
        files.retain(|entry| {
            let region_name = entry.file_name().into_string().unwrap();
            let image_name = format!("{}/{}.png", images_folder, region_name);
            let position = parse_name(&image_name);
            images.insert(position, image_name.clone());

            if let Ok(image_meta) = fs::metadata(&image_name) {
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
            }
        });

        println!(
            "Only {} files need to be updated ({:.2}%)",
            files.len(),
            files.len() as f32 / original_len as f32 * 100.0
        );
    }

    save_images(files, &images_folder, generate_textures);
    if generate_textures {
        save_collage(&images_folder, &images, (16, 16));
    } else {
        save_collage(&images_folder, &images, (1, 1));
    }
}
