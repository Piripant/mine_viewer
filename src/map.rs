use std::fs::File;
use std::io::prelude::*;

use std::collections::HashMap;
use std::io::{BufReader, SeekFrom};

use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use flate2::read::{GzDecoder, ZlibDecoder};

use super::nbt;

const SECTOR_SIZE: u64 = 4096;

const EMPTY_BLOCK: &str = "minecraft:air";

// How many blocks is a section high
const SECTION_SIZE: usize = 16;
// How many blocks is a chunk long/deep
const CHUNK_SIZE: usize = 16;
// How many chunk is a region long/deep
const REGION_SIZE: usize = 32;

pub struct RegionFile {
    reader: BufReader<File>,
}

impl RegionFile {
    pub fn new(file_name: &std::path::Path) -> std::io::Result<RegionFile> {
        let file = File::open(file_name)?;
        let reader = BufReader::new(file);
        Ok(RegionFile { reader })
    }

    // Returns (sector, size)
    pub fn read_header(&mut self) -> std::io::Result<Vec<(u32, u8)>> {
        let mut chunks = Vec::new();
        // Only read the first 4096 bytes
        // Where the sector position and size are stored
        // We dont care about the timestamps for now
        for _ in 0..32 {
            for _ in 0..32 {
                // Format: [3byte offset, 1byte sector size]
                let offset = self.reader.read_u24::<BigEndian>()?;
                let size = self.reader.read_u8()?;
                chunks.push((offset, size as u8));
            }
        }

        Ok(chunks)
    }

    // Returns the chunk nbt data uncompressed but undeserialized
    pub fn read_chunk(&mut self, offset: u32, size: u8) -> std::io::Result<Vec<u8>> {
        self.reader
            .seek(SeekFrom::Start(u64::from(offset) * SECTOR_SIZE))?;

        // This number rapresents the length in bytes with the compression_type u8
        // We subtract one to get the length of only the compressed data
        let _length = self.reader.read_u32::<BigEndian>()? - 1;
        let compression_type = self.reader.read_u8()?;
        let actual_size = u64::from(size) * SECTOR_SIZE;

        // The actual_size - 5 is the size of the compressed data
        // 4 bytes for the length, 1 byte for the compression_type must be subtracted
        let mut compressed_chunk = vec![0; actual_size as usize - 5];
        self.reader.read_exact(&mut compressed_chunk)?;

        let mut uncompressed_chunk = Vec::new();
        match compression_type {
            1 => {
                let mut z = GzDecoder::new(&compressed_chunk[..]);
                z.read_to_end(&mut uncompressed_chunk)?;
            }
            2 => {
                let mut z = ZlibDecoder::new(&compressed_chunk[..]);
                z.read_to_end(&mut uncompressed_chunk)?;
            }
            _ => panic!("Unknown nbt compression type"),
        }

        Ok(uncompressed_chunk)
    }
}

// bytes: bytes buffer, start: start of the number in bits, n: length of the number in bits
fn read_bits(bytes: &[u8], start: usize, n: usize) -> u32 {
    let mut number = 0;
    for j in 0..n {
        let bit = start + j;
        // Get a bit that forms this number (false = 0, true = 1)
        let binary = bytes[bit / 8] & (0x01 << (bit % 8)) != 0;
        if binary {
            number += 2u32.pow(j as u32);
        }
    }
    number
}

#[derive(Debug)]
pub struct ChunkSection {
    // Merge names properties and graphic_props together
    pub names: Vec<String>,
    pub properties: Vec<String>,
    // Useful for rendering the blocks
    pub graphic_props: Vec<String>,
    pub indexes: Vec<usize>,
}

// The map is:
// HashMap<name of the block, HashMap<name of the propriety, index in the model file>>
// It is important that graphical properties are ordered the same way as in the model file
// So that we can search for the correct variant
type GraphPropsMap = HashMap<String, HashMap<String, usize>>;

impl ChunkSection {
    pub fn new(section: &nbt::Compound, graphic_set: &GraphPropsMap) -> (ChunkSection, i8) {
        let mut names = Vec::new();
        let mut indexes = Vec::new();
        let mut properties = Vec::new();
        let mut graphic_props = Vec::new();

        if let Some(palette) = section.get("Palette") {
            let palette = palette.as_list().expect("Could not parse Palette as list");
            for block in palette {
                let block = block
                    .as_compound()
                    .expect("Could not parse block as Compound");
                let name = block["Name"]
                    .as_string()
                    .expect("Could not parse block Name as string");

                let mut prop_list = String::new();
                let graphics = graphic_set
                    .get(name)
                    .expect(&format!(
                        "Error reading the graphic properties for block {}, are you using an old minecraft version for this world?", name));
                // The list of graphical properties, ordered the same way as in the blockstates files
                let mut graphic_list = vec![String::new(); graphics.len()];
                if let Some(block_properties) = &block.get("Properties") {
                    let block_properties = block_properties
                        .as_compound()
                        .expect("Could not parse block_properties as Compound");
                    for (key, value) in block_properties {
                        // This format is convinient when searching the block variant
                        // In the blockstate json files
                        let text = format!(
                            "{}={}",
                            key,
                            value
                                .as_string()
                                .expect("Could not parse block value as String")
                        );
                        prop_list.push_str(&text);
                        // If this property is in the list of graphical properties add it in the right position
                        // To replicate the same order as in the blockstate file
                        if let Some(index) = graphics.get(key) {
                            graphic_list[*index] = text;
                        }
                    }

                    // Remove the last ',' character
                    prop_list.pop();
                }

                // Join the properties in a single string, in the same format as `prop_list`
                let mut graphic_list: String =
                    graphic_list.iter_mut().map(|x| format!("{},", x)).collect();
                // Remove the last ',' character
                graphic_list.pop();

                names.push(name.to_owned());
                properties.push(prop_list);
                graphic_props.push(graphic_list);
            }

            // If 'Palette' was in the nbt, BlockStates will also be there
            // In future versions of Rust use the `if let` chain syntax
            let states = &section["BlockStates"]
                .as_i64_vec()
                .expect("Could not parse BlockStates as i64 vec");
            // Trasform the i64 array in a LittleEndian byte array
            let states: Vec<u8> = states
                .iter()
                .flat_map(|state| {
                    let mut buf = [0; 8];
                    LittleEndian::write_i64(&mut buf, *state);
                    buf.to_vec()
                })
                .collect();

            // There are always 4096 different blocks per section (a section is a cube of blocks 16x16x16 = 4096)
            // So we can calculate the number of bits used for each block like this
            let block_bits = states.len() * 8 / 4096;
            let block_bits = usize::max(block_bits, 4);
            for i in 0..4096 {
                // The start of this number in bits is `i * block_bits`
                let number = read_bits(&states, i * block_bits, block_bits) as usize;
                indexes.push(number);
            }
        }

        // Check that there are no indexes which go out of bounds
        if let Some(max) = indexes.iter().max() {
            assert!(*max == names.len() - 1);
        }

        let y_index = section["Y"].as_i8().expect("Could not parse Y as i8");
        (
            ChunkSection {
                names,
                properties,
                graphic_props,
                indexes,
            },
            *y_index,
        )
    }

    fn get_index(&self, x: usize, y: usize, z: usize) -> usize {
        y * CHUNK_SIZE * CHUNK_SIZE + z * CHUNK_SIZE + x
    }

    // ? Merge this three functions together

    pub fn get_block(&self, x: usize, y: usize, z: usize) -> &str {
        let name_index = self.indexes[self.get_index(x, y, z)];
        &self.names[name_index]
    }

    #[allow(dead_code)]
    pub fn get_prop(&self, x: usize, y: usize, z: usize) -> &str {
        let prop_index = self.indexes[self.get_index(x, y, z)];
        &self.properties[prop_index]
    }

    pub fn get_gprop(&self, x: usize, y: usize, z: usize) -> &str {
        let prop_index = self.indexes[self.get_index(x, y, z)];
        &self.graphic_props[prop_index]
    }
}

#[derive(Debug)]
pub struct Chunk {
    sections: Vec<Option<ChunkSection>>,
}

impl Chunk {
    pub fn new(chunk_nbt: &nbt::Compound, graphic_set: &GraphPropsMap) -> Chunk {
        // Add the sections to the chunk
        let mut sections: Vec<Option<ChunkSection>> = (0..16).map(|_| None).collect();
        let sections_nbt = chunk_nbt["Sections"]
            .as_list()
            .expect("Could not parse Sections as list");
        for section_nbt in sections_nbt {
            let section_nbt = section_nbt
                .as_compound()
                .expect("Could not parse section as Compount");
            let (section, y) = ChunkSection::new(section_nbt, graphic_set);
            // Sometimes there are chunks with the index of -1
            // Which are completely empty
            if y != -1 && !section.names.is_empty() {
                sections[y as usize] = Some(section);
            }
        }

        Chunk { sections }
    }

    pub fn get_block(&self, x: usize, y: usize, z: usize) -> &str {
        let index = y / SECTION_SIZE;
        if let Some(section) = &self.sections[index] {
            section.get_block(x, y % SECTION_SIZE, z)
        } else {
            EMPTY_BLOCK
        }
    }

    #[allow(dead_code)]
    pub fn get_prop(&self, x: usize, y: usize, z: usize) -> &str {
        let index = y / SECTION_SIZE;
        if let Some(section) = &self.sections[index] {
            section.get_prop(x, y % SECTION_SIZE, z)
        } else {
            ""
        }
    }

    pub fn get_gprop(&self, x: usize, y: usize, z: usize) -> &str {
        let index = y / SECTION_SIZE;
        if let Some(section) = &self.sections[index] {
            section.get_gprop(x, y % SECTION_SIZE, z)
        } else {
            ""
        }
    }
}

pub struct Region {
    // There are 32x32 chunks in each region
    chunks: Vec<Option<Chunk>>,
}

impl Region {
    pub fn new(chunks: Vec<Option<Chunk>>) -> Region {
        Region { chunks }
    }
    pub fn new_empty() -> Region {
        Region {
            chunks: (0..REGION_SIZE * REGION_SIZE).map(|_| None).collect(),
        }
    }

    pub fn from_file(
        file_name: &std::path::Path,
        graphic_set: &GraphPropsMap,
    ) -> std::io::Result<Region> {
        let mut region_nbt = RegionFile::new(file_name)?;
        let chunks_nbt = region_nbt.read_header()?;

        let mut chunks = Vec::new();
        for &(offset, size) in &chunks_nbt {
            if offset != 0 && size != 0 {
                if let Ok(chunk) = region_nbt.read_chunk(offset, size) {
                    let mut parser = nbt::NBTParser::new(chunk);

                    let tags = parser.read_compound()?;
                    let tags = tags
                        .as_compound()
                        .expect("Could not read nbt tags from file");
                    let level = tags[""]
                        .as_compound()
                        .expect("Could not parse tags[\"\"] as compound")["Level"]
                        .as_compound()
                        .expect("Could not read Level from nbt tags");

                    let chunk = Chunk::new(&level, graphic_set);
                    chunks.push(Some(chunk));
                } else {
                    println!("Chunk error");
                    chunks.push(None);
                }
            } else {
                chunks.push(None);
            }
        }

        Ok(Region::new(chunks))
    }

    pub fn get_index(&self, x: usize, z: usize) -> usize {
        (z / CHUNK_SIZE) * REGION_SIZE + (x / CHUNK_SIZE)
    }

    pub fn get_block(&self, x: usize, y: usize, z: usize) -> &str {
        let index = self.get_index(x, z);
        if let Some(chunk) = &self.chunks[index] {
            chunk.get_block(x % CHUNK_SIZE, y, z % CHUNK_SIZE)
        } else {
            EMPTY_BLOCK
        }
    }

    #[allow(dead_code)]
    pub fn get_prop(&self, x: usize, y: usize, z: usize) -> &str {
        let index = self.get_index(x, z);
        if let Some(chunk) = &self.chunks[index] {
            chunk.get_prop(x % CHUNK_SIZE, y, z % CHUNK_SIZE)
        } else {
            ""
        }
    }

    pub fn get_gprop(&self, x: usize, y: usize, z: usize) -> &str {
        let index = self.get_index(x, z);
        if let Some(chunk) = &self.chunks[index] {
            chunk.get_gprop(x % CHUNK_SIZE, y, z % CHUNK_SIZE)
        } else {
            ""
        }
    }
}
