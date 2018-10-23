extern crate flacparse;

use std::env;
use std::process::exit;
use std::io;
use std::fs::File;
use std::io::{Read, BufRead, BufReader};

use flacparse::*;

fn analyse_flac_file<R>(f: &mut R) -> Result<(), io::Error>
where R: Read + BufRead {  
    let vorbis_comments = parse(f)?;

    println!("Comments: {:?}", vorbis_comments);
    println!("Title: {}", vorbis_comments.title().unwrap_or_default());
    println!("Artist: {}", vorbis_comments.artist().unwrap_or_default());
    println!("Album: {}", vorbis_comments.album().unwrap_or_default());
    println!("Number: {}", vorbis_comments.tracknumber().unwrap_or_default());

    Ok(())
}

fn main() {
    let stdin = io::stdin();
    if let Some(input) = env::args().nth(1) {
        match input.as_ref() {
            "-" => analyse_flac_file(stdin.lock().by_ref()),
            x => analyse_flac_file(BufReader::new(File::open(x).unwrap()).by_ref()),
        }.unwrap();
    } else {
        eprintln!("Supply the file to dump as the first argument");
        exit(1);
    }
}