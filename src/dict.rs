use crate::word::{Word, WORD_LENGTH};

const WORDLES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/wordles.bin"));
const OTHER_WORDS: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/other_words.bin"));

pub struct DictIterator<'a> {
    prev_word: [u8; WORD_LENGTH],
    remaining_bytes_in_block: &'a [u8],
    remaining_bytes_after_block: &'a [u8],
}

impl<'a> DictIterator<'a> {
    fn of_slice(bytes: &'a [u8]) -> Self {
        DictIterator { prev_word: [0; 5], remaining_bytes_in_block: &[], remaining_bytes_after_block: bytes }
    }

    fn next_start_new_block(&mut self) -> Option<Word> {
        const BLOCK_SIZE: usize = 64;
        debug_assert!(self.remaining_bytes_in_block.is_empty(), "Should only be called when the block is empty");
        if self.remaining_bytes_after_block.len() == 0 {
            None
        } else {
            let (next_block, remaining_after_block) = unsafe {
                assert_eq!(self.remaining_bytes_after_block.len() % BLOCK_SIZE, 0, "Should only have a multiple of the block size");
                debug_assert!(self.remaining_bytes_after_block.len() >= BLOCK_SIZE, "Should not be empty and be a multiple of the block size");
                // TODO: Eventually this will be made unchecked once that's stable
                self.remaining_bytes_after_block.split_at(BLOCK_SIZE)
            };
            let (first_word, remaining_in_block) = next_block.split_at(WORD_LENGTH);
            self.prev_word = first_word.try_into().expect("Must be the right length");
            self.remaining_bytes_in_block = remaining_in_block;
            self.remaining_bytes_after_block = remaining_after_block;
            Some(Word::from(self.prev_word))
        }
    }
}

impl<'a> Iterator for DictIterator<'a> {
    type Item = Word;

    fn next(&mut self) -> Option<Self::Item> {
        match self.remaining_bytes_in_block.split_first() {
            None => self.next_start_new_block(),
            Some((&prefix_len, to_remain)) => {
                if prefix_len == 0xff {
                    self.remaining_bytes_in_block = &[];
                    self.next_start_new_block()
                } else {
                    self.remaining_bytes_in_block = to_remain;
                    let prefix_len = prefix_len as usize;
                    let suffix =  {
                        let suffix_len = WORD_LENGTH - prefix_len;
                        if suffix_len > self.remaining_bytes_in_block.len() {
                            panic!("Invalid data in compiled dictionary")
                        }
                        let (suffix, to_remain) =
                            self.remaining_bytes_in_block.split_at(WORD_LENGTH - prefix_len);
                        self.remaining_bytes_in_block = to_remain;
                        suffix
                    };

                    self.prev_word[prefix_len..].copy_from_slice(suffix);
                    Some(Word::from(self.prev_word))
                }
            },
        }
    }
}

pub fn wordles() -> DictIterator<'static> {
    DictIterator::of_slice(WORDLES)
}

pub fn other_words() -> DictIterator<'static> {
    DictIterator::of_slice(OTHER_WORDS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::io::{BufReader, BufRead};

    fn read_words(from_path: impl AsRef<Path>) -> std::io::Result<Vec<[u8; WORD_LENGTH]>> {
        let read_f = std::fs::File::open(from_path)?;
        let mut read_f = std::io::BufReader::new(read_f);
        let mut buf = Vec::with_capacity(16);
        let mut res = Vec::new();
        loop {
            let read_size = read_f.read_until(b'\n', &mut buf)?;
            if read_size == 0 { break Ok(res) }
            if read_size != (WORD_LENGTH + 1) { panic!("Line of the wrong size!") }
            res.push(buf[..WORD_LENGTH].try_into().unwrap())
        }
    }

    #[test]
    fn check_wordles() {
        let mut actual_wordles = read_words("data/wordles.txt").unwrap();
        actual_wordles.sort_unstable();

        actual_wordles.iter().zip(wordles()).for_each(|(expected, actual)| {
            assert_eq!(Word::from(*expected), actual);
        })
    }

    #[test]
    fn check_other_words() {
        let mut other_words = read_words("data/wordles.txt").unwrap();
        other_words.sort_unstable();

        other_words.iter().zip(wordles()).for_each(|(expected, actual)| {
            assert_eq!(Word::from(*expected), actual);
        })
    }
}
