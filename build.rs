use std::env;
use std::path::Path;
use std::io::{BufRead, Read, Write};

const WORD_LENGTH: usize = 5;

fn read_words(from_path: impl AsRef<Path>) -> std::io::Result<Vec<[u8; WORD_LENGTH]>> {
    let read_f = std::fs::File::open(from_path)?;
    let mut read_f = std::io::BufReader::new(read_f);
    let mut buf = Vec::with_capacity(16);
    let mut res = Vec::new();
    loop {
        let read_size = read_f.read_until(b'\n', &mut buf)?;
        if read_size == 0 { break Ok(res) }
        if read_size != (WORD_LENGTH + 1) { panic!("Line of the wrong size!") }
        res.push(buf[..WORD_LENGTH].try_into().unwrap());
        buf.clear();
    }
}

fn common_prefix_length<T: Eq>(left: &[T], right: &[T]) -> usize {
    left.iter().zip(right.iter()).take_while(|pair| {
        <T as PartialEq>::eq(pair.0, pair.1)
    }).count()
}

fn write_words(dest_path: impl AsRef<Path>, words: &[[u8; WORD_LENGTH]]) -> std::io::Result<()> {
    let mut write_f = std::fs::File::create(dest_path)?;
    const BLOCK_SIZE: usize = 64;
    const WRITE_SIZE: usize = 4 * 1024;
    let mut buf = Vec::with_capacity(WRITE_SIZE);
    let mut words = words.iter().cloned().peekable();
    loop {
        let mut prev_word = match words.next() {
            Some(word) => word,
            None => break,
        };
        let block_end = buf.len() + BLOCK_SIZE;
        buf.extend_from_slice(prev_word.as_slice());
        loop {
            let next_word = match words.peek() {
                Some(word) => word,
                None => break,
            };

            let prefix_len = common_prefix_length(prev_word.as_slice(), next_word.as_slice());
            if buf.len() + 2 + (WORD_LENGTH - prefix_len) > block_end {
                break;
            }
            let word = words.next().expect("Already peeked");
            buf.push(prefix_len as u8);
            buf.extend_from_slice(&word[prefix_len..]);
            prev_word = word;
        }
        buf.push(0xff);
        while buf.len() < block_end {
            buf.push(b' ');
        }

        if buf.len() >= WRITE_SIZE {
            write_f.write_all(buf.as_slice())?;
            buf.clear()
        }
    }

    if buf.len() > 0 {
        write_f.write_all(buf.as_slice())?;
    }

    Ok(())
}

fn prep_dict(out_dir: &Path, sub_path: &str, read_from: &str) {
    println!("cargo:rerun-if-changed={}", read_from);
    let mut all_words = read_words(Path::new(read_from)).expect("Reading file should be ok");
    all_words.sort_unstable();
    write_words(out_dir.join(sub_path), all_words.as_slice()).expect("Writing file should be ok");
}

fn main() {
    let out_dir = &env::var_os("OUT_DIR").unwrap();
    let out_dir = Path::new(out_dir);
    prep_dict(out_dir, "wordles.bin", "data/wordles.txt");
    prep_dict(out_dir, "other_words.bin", "data/other_words.txt");
    println!("cargo:rerun-if-changed=build.rs");
}