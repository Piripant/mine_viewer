mod loader;
mod map;
mod nbt;
mod renderer;

use std::fs;

fn folder_name(path: &str) -> String {
    path.replace('/', ":")
}

fn main() {
    let yaml = clap::load_yaml!("cli.yml");
    let matches = clap::App::from_yaml(yaml).get_matches();

    // Get the command line arguments
    let generate_textures = matches.is_present("textures");
    let update = matches.is_present("update");
    let region_folder = matches.value_of("world").unwrap_or("world").to_owned() + "/region";

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

    let images_folder = format!("images/{}", folder_name(&region_folder));

    // Start the rendering
    std::fs::create_dir_all(&images_folder).unwrap_or_default();

    let mut files: Vec<_> = fs::read_dir(region_folder)
        .unwrap()
        .map(|entry| entry.unwrap())
        .collect();

    // Get a list of all files that need updating
    if update {
        let original_len = files.len();
        files.retain(|entry| {
            let region_name = entry.file_name().into_string().unwrap();
            let image_name = format!("{}/{}.png", images_folder, region_name);

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

        let delta = original_len - files.len();
        if delta > 0 {
            println!(
                "Only {} files need to be updated ({:.2}%)",
                delta,
                files.len() as f32 / original_len as f32 * 100.0
            );
        }
    }

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
            renderer::image_chunk_textures(&region, &ignore, &mut textures, &image_name);
        } else {
            renderer::image_chunk(&region, &ignore, &mut textures, &image_name);
        }
    }
}
