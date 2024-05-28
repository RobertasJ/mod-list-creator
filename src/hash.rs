use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::process::exit;

pub struct Buffer(Vec<u8>);

impl Buffer {
    fn new(size: usize) -> Self {
        Buffer(vec![0; size])
    }

    fn data(&self) -> &[u8] {
        &self.0
    }

    fn data_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }

    fn size(&self) -> usize {
        self.0.len()
    }
}

pub fn get_jar_contents(jar_file_path: &str) -> Buffer {
    let mut jar_file = File::open(jar_file_path).unwrap_or_else(|err| {
        eprintln!("failed opening mod file: {}", err.to_string());
        exit(1);
    });

    let buffer_size = get_file_size(&mut jar_file).unwrap_or_else(|err| {
        eprintln!("failed getting file size: {}", err.to_string());
        exit(1);
    });

    let mut buffer = Buffer::new(buffer_size);
    jar_file.read_exact(buffer.data_mut()).unwrap_or_else(|err| {
        eprintln!("failed to load: {}", err.to_string());
        exit(1);
    });

    buffer
}

fn get_file_size(file: &mut File) -> std::io::Result<usize> {
    file.seek(SeekFrom::End(0))?;
    let size = file.stream_position()?;
    file.seek(SeekFrom::Start(0))?;
    Ok(size as usize)
}

pub fn compute_hash(buffer: &Buffer) -> u32 {
    const MULTIPLEX: u32 = 1540483477;
    let length = buffer.size();
    let mut num1 = length as u32;

    num1 = compute_normalized_length(buffer);

    let mut num2 = 1 ^ num1;
    let mut num3 = 0;
    let mut num4 = 0;

    for &b in buffer.data() {
        if !is_whitespace_character(b) {
            num3 |= (b as u32) << num4;
            num4 += 8;
            if num4 == 32 {
                let num6 = num3.wrapping_mul(MULTIPLEX);
                let num7 = (num6 ^ (num6 >> 24)).wrapping_mul(MULTIPLEX);
                num2 = num2.wrapping_mul(MULTIPLEX) ^ num7;
                num3 = 0;
                num4 = 0;
            }
        }
    }

    if num4 > 0 {
        num2 = (num2 ^ num3).wrapping_mul(MULTIPLEX);
    }

    let num6 = (num2 ^ (num2 >> 13)).wrapping_mul(MULTIPLEX);
    num6 ^ (num6 >> 15)
}

fn compute_normalized_length(buffer: &Buffer) -> u32 {
    let mut num1 = 0;
    let length = buffer.size();

    for &b in buffer.data() {
        if !is_whitespace_character(b) {
            num1 += 1;
        }
    }

    num1
}

fn is_whitespace_character(b: u8) -> bool {
    b == 9 || b == 10 || b == 13 || b == 32
}