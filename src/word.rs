use std::error::Error;
use std::fmt::{Display, Formatter};
use pyo3::{FromPyObject, PyAny, PyResult};

pub const WORD_LENGTH: usize = 5;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Word([u8; WORD_LENGTH]);

impl From<[u8; 5]> for Word {
    fn from(x: [u8; 5]) -> Self {
        Word(x)
    }
}

impl Word {
    pub fn bytes(&self) -> &[u8; 5] {
        &self.0
    }
}

impl TryFrom<&str> for Word {
    type Error = WordOfStringError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let bytes = value.as_bytes();
        if bytes.len() != WORD_LENGTH {
            return Err(WordOfStringError::WrongLength)
        } else if !bytes.iter().all(|b| (b'a'..=b'z').contains(b)) {
            return Err(WordOfStringError::InvalidLetter)
        } else {
            return Ok(Word(bytes.try_into().unwrap()))
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum WordOfStringError {
    WrongLength,
    InvalidLetter
}

impl Display for WordOfStringError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            WordOfStringError::WrongLength => write!(f, "Wrong length: word must be exactly {} letters", WORD_LENGTH),
            WordOfStringError::InvalidLetter => write!(f, "Invalid letter: all letters in word must be a..z"),
        }
    }
}

impl Error for WordOfStringError {

}

impl WordOfStringError {
    fn into_value_error(&self) -> pyo3::PyErr {
        pyo3::exceptions::PyValueError::new_err(self.to_string())
    }
}

#[derive(Clone)]
pub struct WordIter {
    word: Word, position: u8
}

impl Iterator for WordIter {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        self.word.0.get(self.position as usize).map(|x| {
            self.position = self.position.wrapping_add(1);
            *x
        })
    }

    fn for_each<F>(self, mut f: F) where F: FnMut(Self::Item) {
        self.word.0[self.position as usize..].iter().for_each(|&x| f(x));
    }
}

impl IntoIterator for Word {
    type Item = <Self::IntoIter as Iterator>::Item;
    type IntoIter = WordIter;

    fn into_iter(self) -> Self::IntoIter {
        WordIter { word: self, position: 0 }
    }
}

impl<'source> FromPyObject<'source> for Word {
    fn extract(ob: &'source PyAny) -> PyResult<Self> {
        Word::try_from(<&str as FromPyObject>::extract(ob)?)
            .map_err(|err| err.into_value_error())
    }
}
