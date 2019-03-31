mod loader;
mod map;
mod nbt;
mod renderer;

use std::fs;
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

    // Get the command line arguments
    let generate_textures = matches.is_present("textures");
    let update = matches.is_present("update");
    let region_folder = matches.value_of("region").unwrap_or("region");

    // Load all the settings
    let ignore = loader::load_ignore_blocks().unwrap_or_else(|err| {
        println!("Error loading ignore blocks file: {}", err);
        std::process::exit(0)
    });
    let graphic_set = loader::load_graphic_props().unwrap_or_else(|err| {
        println!("Error loading files in blockstates: {}", err);
        std::process::exit(0)
    });
    let mut textures =
        loader::TextureLoader::new(loader::load_biome_blocks().unwrap_or_else(|err| {
            println!("Error loading biome blocks file: {}", err);
            std::process::exit(0)
        }));

    // Start the rendering
    std::fs::create_dir("images").unwrap_or_default();
    let update = if let Ok(text) = fs::read_to_string("images/origin.txt") {
        if text == region_folder {
            update
        } else {
            false
        }
    } else {
        false
    };

    // Indicate which region folder originated this images
    fs::write("images/origin.txt", region_folder.as_bytes()).unwrap();

    let files: Vec<std::fs::DirEntry> = fs::read_dir(region_folder)
        .unwrap()
        .map(|entry| entry.unwrap())
        .collect();
    for (i, entry) in files.iter().enumerate() {
        let region_name = entry.file_name().into_string().unwrap();
        let image_name = format!("images/{}.png", region_name);

        let generate = if !update {
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
            let region = map::Region::from_file(&entry.path(), &graphic_set)
                .unwrap_or_else(|_| map::Region::new_empty());

            if generate_textures {
                renderer::image_chunk_textures(&region, &ignore, &mut textures, &image_name);
            } else {
                renderer::image_chunk(&region, &ignore, &mut textures, &image_name);
            }
        } else {
            print!("Skipping {}, nothing new", region_name);
        }

        println!(
            " | {}/{} ({:.2}%)",
            i + 1,
            files.len(),
            (i + 1) as f32 / files.len() as f32 * 100.0
        );
    }
}
