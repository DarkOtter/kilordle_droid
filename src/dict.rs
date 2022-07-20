use rayon::iter::IntoParallelIterator;
use rayon::iter::plumbing::{Folder, UnindexedConsumer, UnindexedProducer};
use rayon::prelude::ParallelIterator;
use crate::word::{Word, WORD_LENGTH};

const BLOCK_SIZE: usize = 64;

const WORDLES: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/wordles.bin"));
const OTHER_WORDS: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/other_words.bin"));

#[derive(Clone, Debug)]
pub struct DictIterator<'a> {
    prev_word: [u8; WORD_LENGTH],
    remaining_bytes_in_block: &'a [u8],
    remaining_bytes_after_block: &'a [u8],
}

impl<'a> DictIterator<'a> {
    fn of_slice(bytes: &'a [u8]) -> Self {
        DictIterator { prev_word: [0; 5], remaining_bytes_in_block: &[], remaining_bytes_after_block: bytes }
    }

    fn split(mut self) -> (Self, Option<Self>) {
        if self.remaining_bytes_after_block.is_empty() {
            return (self, None)
        }
        assert_eq!(self.remaining_bytes_after_block.len() % BLOCK_SIZE, 0, "Remaining data after block should be a multiple of block size");
        let n_blocks_remain = self.remaining_bytes_after_block.len() / BLOCK_SIZE;
        let keep_n_blocks = n_blocks_remain / 2;
        let (for_this_iterator, for_other) = self.remaining_bytes_after_block.split_at(keep_n_blocks * BLOCK_SIZE);
        self.remaining_bytes_after_block = for_this_iterator;
        (self, Some(DictIterator::of_slice(for_other)))
    }

    fn next_start_new_block(&mut self) -> Option<Word> {
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

#[derive(Clone, Debug)]
pub struct ParallelDictIterator<'a>(DictIterator<'a>);


impl<'a> UnindexedProducer for ParallelDictIterator<'a> {
    type Item = <DictIterator<'a> as Iterator>::Item;

    fn split(self) -> (Self, Option<Self>) {
        let (new_self, other) = self.0.split();
        (ParallelDictIterator(new_self), other.map(|other| ParallelDictIterator(other)))
    }

    fn fold_with<F>(self, folder: F) -> F where F: Folder<Self::Item> {
        folder.consume_iter(self.0)
    }
}

impl<'a> ParallelIterator for ParallelDictIterator<'a> {
    type Item = <Self as UnindexedProducer>::Item;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result where C: UnindexedConsumer<Self::Item> {
        rayon::iter::plumbing::bridge_unindexed(self, consumer)
    }
}

impl<'a> IntoParallelIterator for DictIterator<'a> {
    type Iter = ParallelDictIterator<'a>;
    type Item = <Self as Iterator>::Item;

    fn into_par_iter(self) -> Self::Iter {
        ParallelDictIterator(self)
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
    use std::fs::read;
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
            res.push(buf[..WORD_LENGTH].try_into().unwrap());
            buf.clear();
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

    // #[test]
    // fn check_wordles_parallel() {
    //     let mut actual_wordles = read_words("data/wordles.txt").unwrap();
    //     actual_wordles.sort_unstable();
    //     let mut read_wordles: Vec<_> = wordles().into_par_iter().collect();
    //     read_wordles.sort_unstable();
    //
    //     actual_wordles.iter().zip(wordles()).for_each(|(expected, actual)| {
    //         assert_eq!(Word::from(*expected), actual);
    //     })
    // }

    #[test]
    fn check_other_words() {
        let mut other_words = read_words("data/wordles.txt").unwrap();
        other_words.sort_unstable();

        other_words.iter().zip(wordles()).for_each(|(expected, actual)| {
            assert_eq!(Word::from(*expected), actual);
        })
    }
}
