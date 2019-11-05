use std::io;
use std::io::prelude::*;
use std::io::Cursor;

use byteorder::{BigEndian, ReadBytesExt};
use std::collections::HashMap;

//pub type Result = std::result::Result<T, Error>;

//pub enum Error {

//}

// An NBT compound can be espress as an HashMap
pub type Compound = HashMap<String, Tag>;

// There is no need to have an End tag
// As the parsed structure doesn't use them
#[derive(Debug)]
pub enum Tag {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<Tag>),
    Compound(Compound),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl Tag {
    #[allow(dead_code)]
    pub fn as_compound(&self) -> Option<&Compound> {
        if let Tag::Compound(comp) = self {
            Some(comp)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn as_i8(&self) -> Option<&i8> {
        if let Tag::Byte(n) = self {
            Some(n)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn as_i16(&self) -> Option<&i16> {
        if let Tag::Short(n) = self {
            Some(n)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn as_i32(&self) -> Option<&i32> {
        if let Tag::Int(n) = self {
            Some(n)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn as_i64(&self) -> Option<&i64> {
        if let Tag::Long(n) = self {
            Some(n)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn as_i8_vec(&self) -> Option<&Vec<i8>> {
        if let Tag::ByteArray(n) = self {
            Some(n)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn as_string(&self) -> Option<&str> {
        if let Tag::String(n) = self {
            Some(n)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn as_list(&self) -> Option<&Vec<Tag>> {
        if let Tag::List(n) = self {
            Some(n)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn as_i32_vec(&self) -> Option<&Vec<i32>> {
        if let Tag::IntArray(n) = self {
            Some(n)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn as_i64_vec(&self) -> Option<&Vec<i64>> {
        if let Tag::LongArray(n) = self {
            Some(n)
        } else {
            None
        }
    }
}

pub struct NBTParser {
    bytes: Cursor<Vec<u8>>,
}

impl NBTParser {
    pub fn new(bytes: Vec<u8>) -> NBTParser {
        NBTParser {
            bytes: Cursor::new(bytes),
        }
    }

    pub fn read_id(&mut self, id: u8) -> io::Result<Tag> {
        Ok(match id {
            1 => self.read_byte()?,
            2 => self.read_short()?,
            3 => self.read_int()?,
            4 => self.read_long()?,
            5 => self.read_float()?,
            6 => self.read_double()?,
            7 => self.read_byte_array()?,
            8 => self.read_string()?,
            9 => self.read_list()?,
            10 => self.read_compound()?,
            11 => self.read_int_array()?,
            12 => self.read_long_array()?,
            _ => panic!("Unknown tag type"),
        })
    }

    // At the root of each file there is a Compound
    // We can use this function to recursively parse any file
    pub fn read_compound(&mut self) -> io::Result<Tag> {
        let mut comp = Compound::new();
        while let Ok(tag_id) = self.bytes.read_u8() {
            // The end of the compound
            if tag_id == 0 {
                return Ok(Tag::Compound(comp));
            }

            // Get the name of the field we are about to read
            let name = self.read_name()?;
            comp.insert(name, self.read_id(tag_id)?);
        }

        Ok(Tag::Compound(comp))
    }

    // Reads a tag name
    fn read_name(&mut self) -> io::Result<String> {
        let name_length = self.bytes.read_u16::<BigEndian>()?;
        let mut name = vec![0; name_length as usize];
        self.bytes.read_exact(&mut name)?;
        Ok(String::from_utf8(name).expect("Could not read name as utf8"))
    }

    fn read_byte(&mut self) -> io::Result<Tag> {
        Ok(Tag::Byte(self.bytes.read_i8()?))
    }

    fn read_short(&mut self) -> io::Result<Tag> {
        Ok(Tag::Short(self.bytes.read_i16::<BigEndian>()?))
    }

    fn read_int(&mut self) -> io::Result<Tag> {
        Ok(Tag::Int(self.bytes.read_i32::<BigEndian>()?))
    }

    fn read_long(&mut self) -> io::Result<Tag> {
        Ok(Tag::Long(self.bytes.read_i64::<BigEndian>()?))
    }

    fn read_float(&mut self) -> io::Result<Tag> {
        Ok(Tag::Float(self.bytes.read_f32::<BigEndian>()?))
    }

    fn read_double(&mut self) -> io::Result<Tag> {
        Ok(Tag::Double(self.bytes.read_f64::<BigEndian>()?))
    }

    fn read_byte_array(&mut self) -> io::Result<Tag> {
        let length = self.bytes.read_i32::<BigEndian>()?;

        let mut bytes = vec![0; length as usize];
        for byte in &mut bytes {
            *byte = self.bytes.read_i8()?;
        }

        Ok(Tag::ByteArray(bytes))
    }

    fn read_string(&mut self) -> io::Result<Tag> {
        let length = self.bytes.read_u16::<BigEndian>()?;
        let mut string = vec![0; length as usize];
        self.bytes.read_exact(&mut string)?;
        let string = String::from_utf8(string).expect("Could not read string as utf8");

        Ok(Tag::String(string))
    }

    fn read_list(&mut self) -> io::Result<Tag> {
        let id = self.bytes.read_u8()?;
        let length = self.bytes.read_i32::<BigEndian>()?;

        let mut tags = Vec::new();
        for _ in 0..length {
            tags.push(self.read_id(id)?);
        }

        Ok(Tag::List(tags))
    }

    fn read_int_array(&mut self) -> io::Result<Tag> {
        let length = self.bytes.read_i32::<BigEndian>()?;

        let mut bytes = vec![0; length as usize];
        for byte in &mut bytes {
            *byte = self.bytes.read_i32::<BigEndian>()?;
        }

        Ok(Tag::IntArray(bytes))
    }

    fn read_long_array(&mut self) -> io::Result<Tag> {
        let length = self.bytes.read_i32::<BigEndian>()?;

        let mut bytes = vec![0; length as usize];
        for byte in &mut bytes {
            *byte = self.bytes.read_i64::<BigEndian>()?;
        }

        Ok(Tag::LongArray(bytes))
    }
}
