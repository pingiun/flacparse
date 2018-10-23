extern crate byteorder;

use std::io::{Result, Error, ErrorKind};
use std::io::prelude::*;
use std::collections::HashMap;
use std::ops::Deref;

use byteorder::{ReadBytesExt, LittleEndian, BigEndian, ByteOrder};

/// This trait allows for different metadata specifications to be accessed by the same functions
pub trait MusicData<'a> {
    /// Get the title of a track
    fn title(&'a self) -> Option<&'a str>;
    /// Get the artist of a track
    fn artist(&'a self) -> Option<&'a str>;
    /// Get the album of a track
    fn album(&'a self) -> Option<&'a str>;
    /// Get the track number
    /// Note: this is a string because many metadata specifications allow for tracknumbers like
    /// A3 (side a, track 3)
    fn tracknumber(&'a self) -> Option<&'a str>;
    /// Get a map with all music data
    fn map(self) -> HashMap<String, String>;
}

/// Represents a Vorbis comment block
#[derive(Debug)]
pub struct VorbisMetadata {
    vendor_string: String,
    user_comments: HashMap<String, String>,
}

impl<'a> MusicData<'a> for VorbisMetadata {
    fn title(&'a self) -> Option<&'a str> {
        self.user_comments.get("TITLE").map(|x| x.deref())
    }
    fn artist(&'a self) -> Option<&'a str> {
        self.user_comments.get("ARTIST").map(|x| x.deref())
    }
    fn album(&'a self) -> Option<&'a str> {
        self.user_comments.get("ALBUM").map(|x| x.deref())
    }
    fn tracknumber(&'a self) -> Option<&'a str> {
        self.user_comments.get("TRACKNUMBER").map(|x| x.deref())
    }
    fn map(self) -> HashMap<String, String> {
        self.user_comments
    }
}

pub trait MusicDataParser<'a, M> 
where M: MusicData<'a> {
    fn parse(&mut self) -> Result<M>;
}

#[derive(Debug)]
pub struct MusicMetaData {
    map: HashMap<String, String>,
}

impl<'a> MusicData<'a> for MusicMetaData {
    fn title(&'a self) -> Option<&'a str> {
        self.map.get("TITLE").map(|x| x.deref())
    }
    fn artist(&'a self) -> Option<&'a str> {
        self.map.get("ARTIST").map(|x| x.deref())
    }
    fn album(&'a self) -> Option<&'a str> {
        self.map.get("ALBUM").map(|x| x.deref())
    }
    fn tracknumber(&'a self) -> Option<&'a str> {
        self.map.get("TRACKNUMBER").map(|x| x.deref())
    }
    fn map(self) -> HashMap<String, String> {
        self.map
    }
}

impl<'a> From<VorbisMetadata> for MusicMetaData {
    fn from(c: VorbisMetadata) -> Self {
        MusicMetaData {map: c.map()}
    }
}

pub struct FlacParser<'a, R> 
where R: 'a + Read + BufRead {
    file: &'a mut R,
}

impl<'a, R> FlacParser<'a, R>
where R: Read + BufRead {
    pub fn new(file: &'a mut R) -> Result<FlacParser<R>> {
        if is_flac_file(file.by_ref())? {
            Ok(FlacParser{file: file})
        } else {
            Err(Error::new(ErrorKind::InvalidData, "could not parse as a flac file"))
        }
    }
}

impl<'a, 'b, R> MusicDataParser<'a, VorbisMetadata> for FlacParser<'b, R>
where R: Read + BufRead {
    fn parse(&mut self) -> Result<VorbisMetadata> {
        search_comment_block(self.file)
    }
}

pub fn parse<'a, R>(file: &mut R) -> Result<MusicMetaData>
where R: Read + BufRead {
    if let Ok(mut fp) = FlacParser::new(file) {
        fp.parse().map(|x| x.into())
    } else {
        Err(Error::new(ErrorKind::InvalidData, "could not parse any metadata"))
    }
}

/// Returns true if the reader is a FLAC file
fn is_flac_file<R>(file: &mut R) -> Result<bool>
where R: Read {
    let mut buffer = [0; 4];
    file.read_exact(&mut buffer)?;
    Ok(buffer == "fLaC".as_bytes())
}

/// Searches for a vorbis comment block in the metadata blocks of a flac file
/// 
/// This function assumes that the first 4 bytes of the flac file have been consumed
fn search_comment_block<R>(file: &mut R) -> Result<VorbisMetadata>
where R: Read + BufRead {
    loop {
        let (last, blocktype, size) = {
            let mut block_header_buf = [0; 4];
            file.read_exact(&mut block_header_buf)?;
            let block_header = block_header_buf[0];
            block_header_buf[0] = 0;
            (block_header >> 7 == 1, block_header & 0b0111111, BigEndian::read_u32(&block_header_buf))
        };
        if last {
            return Err(Error::new(ErrorKind::UnexpectedEof, "no comment block"));
        }
        if blocktype == 4 {
            return parse_vorbis_comments(file.by_ref());
        }
        file.consume(size as usize);
    }
}

/// Parses vorbis comments if the reader is positioned at the start of the comment block
fn parse_vorbis_comments<R>(file: &mut R) -> Result<VorbisMetadata> 
where R: Read {
    // Vorbis comments support vendor strings
    let vendor_string = {
        let length = file.read_u32::<LittleEndian>()?;
        read_n(file.by_ref(), length as u64)?
    };

    let ncomments = file.read_u32::<LittleEndian>()?;
    let mut comments = HashMap::new();

    // Read all the lines into a map
    for _ in 0..ncomments {
        let length = file.read_u32::<LittleEndian>()?;

        let mut split: Vec<String> = read_n(file.by_ref(), length as u64)?.split('=').map(|x| x.to_string()).collect();

        // If this assertion fails, the flac file is malformed
        if split.len() != 2 {
            return Err(Error::new(ErrorKind::InvalidData, "malformed FLAC file, could not split user comment"));
        }
        comments.insert(split.remove(0), split.remove(0));
    }

    Ok(VorbisMetadata{vendor_string: vendor_string, user_comments: comments})
}

/// Read n bytes from the reader and construct it into a string
fn read_n<R>(reader: R, bytes_to_read: u64) -> Result<String>
where R: Read {
    let mut buf = String::new();
    let mut chunk = reader.take(bytes_to_read);
    let n = chunk.read_to_string(&mut buf)?;
    assert_eq!(bytes_to_read as usize, n);
    Ok(buf)
}
