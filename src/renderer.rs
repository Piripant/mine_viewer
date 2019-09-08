use image::ImageBuffer;
use std::collections::HashSet;

use super::loader;
use super::map;

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

fn tint_height(avg: &mut [u8; 3], height: u8) {
    for x in avg {
        *x = u32::min(*x as u32 * height as u32 / 255, 255) as u8;
    }
}

pub fn image_chunk(
    region: &map::Region,
    ignore: &HashSet<String>,
    textures: &mut loader::TextureLoader,
) -> image::RgbImage {
    ImageBuffer::from_fn(16 * 32, 16 * 32, |x_block, z_block| {
        let (x_block, z_block) = (x_block as usize, z_block as usize);
        for y in (0..256).rev() {
            let candidate = region.get_block(x_block, y, z_block);
            if !ignore.contains(candidate) {
                let properties = region.get_gprop(x_block, y, z_block);
                if let Some(index) = textures.load(candidate, properties) {
                    let (_, _, mut avg) = textures.get_texture(index);
                    //tint_height(&mut avg, y as u8);
                    return image::Rgb(avg);
                }
            }
        }
        image::Rgb([0, 0, 0])
    })
}

pub fn image_chunk_textures(
    region: &map::Region,
    ignore: &HashSet<String>,
    textures: &mut loader::TextureLoader,
) -> image::RgbaImage {
    let mut img = ImageBuffer::new(16 * 32 * 16, 16 * 32 * 16);
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

    img
}
