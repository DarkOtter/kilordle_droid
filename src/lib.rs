#[cfg(test)]
#[macro_use]
extern crate lazy_static;

use std::ops::Deref;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use word::{Word, WORD_LENGTH};


// TODO: Reduce visibility
pub mod word;
pub mod dict;

type GuessHistory = Vec<Word>;

const MAX_SCORE: u8 = 3 * (WORD_LENGTH as u8);

fn add_scores(a: u8, b:u8) -> u8 {
    if cfg!(debug) {
        if a > MAX_SCORE || b > MAX_SCORE { panic!("Score value out of range") }
        if b > (MAX_SCORE - a) { panic!("Score addition overflow") }
    }
    a.wrapping_add(b)
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct ScoringState {
    word: Word,
    score_at_position: [u8; WORD_LENGTH],
}

impl ScoringState {
    fn for_word(word: Word) -> Self {
        ScoringState { word, score_at_position: [0; 5]}
    }

    fn add_history_item(&mut self, guess: Word) {
        let ScoringState { word, score_at_position } = self;
        let word = *word;
        let guess = guess.bytes();
        score_at_position.iter_mut().zip(word.bytes().iter()).enumerate().for_each(|(i, (score_at_position, &word_letter))| {
            if word_letter == guess[i] {
                *score_at_position = 3
            } else if guess.iter().any(|&guess_letter| guess_letter == word_letter) && *score_at_position < 1 {
                *score_at_position = 1
            }
        })
    }

    fn add_history_items(&mut self, guesses: &[Word]) {
        guesses.iter().for_each(|guess| self.add_history_item(*guess))
    }

    fn current_score(&self) -> u8 {
        self.score_at_position.iter().sum()
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
enum LetterMatch {
    Nothing = 0,
    Partial,
    Exact
}

#[derive(Copy, Clone, PartialEq, Eq)]
struct GuessResult([LetterMatch; WORD_LENGTH]);

impl GuessResult {
    fn is_possible(&self, guess: Word, word: Word) -> bool {
        let guess = guess.bytes();
        let mut word = *word.bytes();

        // Exact matches
        for (i, &r) in self.0.iter().enumerate() {
            match r {
                LetterMatch::Nothing | LetterMatch::Partial => continue,
                LetterMatch::Exact => {
                    if guess[i] == word[i] {
                        word[i] = b' '
                    } else {
                        return false
                    }
                },
            }
        }

        // Partial matches
        for (i, &r) in self.0.iter().enumerate() {
            match r {
                LetterMatch::Nothing | LetterMatch::Exact => continue,
                LetterMatch::Partial => {
                    if guess[i] == word[i] { return false; }
                    match word.iter().position(|&x| x == guess[i]) {
                        None => return false,
                        Some(idx) => word[idx] = b' ',
                    }
                },
            }
        }

        // No matches
        for (i, &r) in self.0.iter().enumerate() {
            match r {
                LetterMatch::Partial | LetterMatch::Exact => continue,
                LetterMatch::Nothing => {
                    if word.iter().any(|&x| x == guess[i]) {
                        return false
                    }
                },
            }
        }

        true
    }

    fn history_is_possible(guess_history: &[Word], result_history: &[GuessResult], word: Word) -> bool {
        guess_history.iter().zip(result_history.iter()).all(|(&guess, &result)| result.is_possible(guess, word))
    }
}


fn pick_next_guess_inner(guess_history: &[Word], visible_results: &[impl Deref<Target = [GuessResult]>], n_remaining_words: usize) -> Result<Word, PyErr> {
    struct InfoForHistory<'a> {
        result_history: &'a [GuessResult],
        possible_words: Vec<ScoringState>
    }

    let n_invisible_words = match n_remaining_words.checked_sub(visible_results.len()) {
        Some(x) => x,
        None => return Err(PyValueError::new_err("Number of remaining words is insufficient")),
    };

    if visible_results.iter().any(|x| x.len() != guess_history.len()) {
        return Err(PyValueError::new_err("Length of histories are different"))
    }

    let mut possible_visible_words: Vec<_> =
        visible_results.iter().map(|x| InfoForHistory { result_history: x.deref(), possible_words: Vec::new() }).collect();
    let mut possible_invisible_words = Vec::new();
    dict::wordles()
        .map(|word| {
            let mut state = ScoringState::for_word(word);
            state.add_history_items(guess_history);
            state
        })
        .filter(|word| word.current_score() < MAX_SCORE)
        .for_each(|word| {
            possible_invisible_words.push(word);
            possible_visible_words.iter_mut().for_each(|visible_word| {
                if GuessResult::history_is_possible(guess_history, visible_word.result_history, word.word) {
                    visible_word.possible_words.push(word)
                }
            });
        });

    if let Some(maximum_invisible_score) = possible_visible_words.iter().map(|visible_word| visible_word.possible_words.iter().map(|word| word.current_score()).max().unwrap_or(MAX_SCORE)).min() {
        possible_invisible_words.retain(|word| word.current_score() <= maximum_invisible_score);
    }

    let possible_visible_words = possible_visible_words;
    let possible_invisible_words = possible_invisible_words;

    fn average_score(possible_words: &Vec<ScoringState>, extra_guess: Word) -> f64 {
        let total_score_increase =
            possible_words.iter().map(|state| {
                let mut state = state.clone();
                state.add_history_item(extra_guess);
                state.current_score() as u64
            }).sum::<u64>();
        (total_score_increase as f64) / (possible_words.len() as f64)
    }

    let mut res: Option<(Word, f64)> = None;
    dict::wordles().chain(dict::other_words()).for_each(|guess| {
        let visible_score_increase =
            possible_visible_words.iter().map(|visible_word| {
                average_score(&visible_word.possible_words, guess)
            }).sum::<f64>();
        let invisible_score_increase =
            average_score(&possible_invisible_words, guess)
                * n_invisible_words as f64;
        let score_increase = visible_score_increase + invisible_score_increase;
        if res.map_or(true, |res|  score_increase > res.1) {
            res = Some((guess, score_increase))
        }
    });

    res.map(|word| word.0).ok_or_else(|| {
        pyo3::exceptions::PyRuntimeError::new_err("Failed to find any words to be possible guesses")
    })
}

impl GuessResult {
    fn from_str_for_py(s: &str) -> PyResult<Self> {
        let s = s.as_bytes();
        if s.len() != WORD_LENGTH {
            return Err(pyo3::exceptions::PyValueError::new_err(format!("Wrong length: guess result must be exactly {} characters", WORD_LENGTH)));
        }
        let mut res = [LetterMatch::Nothing; WORD_LENGTH];
        for (res, &b) in res.iter_mut().zip(s.iter()) {
            if b == b' ' { continue;
            } else if b == b'o' {
                *res = LetterMatch::Partial;
            } else if b == b'O' {
                *res = LetterMatch::Exact;
            } else {
                return Err(pyo3::exceptions::PyValueError::new_err("Invalid character: guess result must be five characters which are all either ' ' for no match, 'o' for partial match or 'O' for exact match"));
            }
        }
        Ok(GuessResult(res))
    }
}

impl<'source> FromPyObject<'source> for GuessResult {
    fn extract(ob: &'source PyAny) -> PyResult<Self> {
        GuessResult::from_str_for_py(<&str as FromPyObject>::extract(ob)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn word(s: &str) -> Word {
        s.try_into().unwrap()
    }

    fn concat_sort(vectors: &[&[Word]]) -> Vec<Word> {
        let (first, others) = match vectors.split_first() {
            Some((x, xs)) => (*x, xs),
            None => return Vec::new(),
        };
        let mut res = first.to_vec();
        others.into_iter().for_each(|&other| res.extend_from_slice(other));
        res.sort_unstable();
        res
    }

    lazy_static! {
        static ref WORDLES: Vec<Word> = dict::wordles().collect();
        static ref OTHER_WORDS: Vec<Word> = dict::wordles().collect();
        static ref ALL_WORDS: Vec<Word> = concat_sort(&[WORDLES.as_slice(), OTHER_WORDS.as_slice()]);
    }


    fn wordles() -> impl Strategy<Value=Word> {
        proptest::sample::select(WORDLES.as_slice())
    }

    fn all_words() -> impl Strategy<Value=Word> {
        proptest::sample::select(ALL_WORDS.as_slice())
    }

    fn guess_history() -> impl Strategy<Value=GuessHistory> {
        proptest::collection::vec(all_words(), 0..=40)
    }


    #[test]
    fn test_score_examples() {
        fn scoring_state(guesses: &[&str], code_word: &str) -> [u8; WORD_LENGTH] {
            let mut state = ScoringState::for_word(word(code_word));
            guesses.iter().for_each(|&guess| state.add_history_item(word(guess)));
            state.score_at_position
        }

        let history = &["hello", "world"];
        assert_eq!(scoring_state(history, "hello"), [3, 3, 3, 3, 3]);
        assert_eq!(scoring_state(history, "holds"), [3, 3, 3, 1, 0]);
        assert_eq!(scoring_state(history, "daair"), [1, 0, 0, 0, 1]);
    }

    fn result(str: &str) -> GuessResult {
        GuessResult::from_str_for_py(str).unwrap()
    }

    #[test]
    fn test_result_possible_examples() {
        fn possible(r: &str, guess: &str, the_word: &str) -> bool {
            result(r).is_possible(word(guess), word(the_word))
        }
        assert_eq!(possible("     ", "deair", "stoln"), true);
        assert_eq!(possible("     ", "deair", "hello"), false);
        assert_eq!(possible(" O   ", "deair", "hello"), true);
        assert_eq!(possible("  oO ", "stoln", "hello"), true);
        assert_eq!(possible("  oO ", "stoln", "hello"), true);
        assert_eq!(possible("   o ", "aabee", "hello"), true);
        assert_eq!(possible("    o", "aabee", "hello"), true);
    }

    // #[test]
    // fn test_pick_next_guess_example() {
    //     let visible_results: &[Vec<GuessResult>] = &[];
    //     assert_eq!(pick_next_guess_inner(&[], visible_results, 1000).unwrap(), word("cigar"))
    // }
}



/// Finds a next guess that can be made in a game of kilordle.
#[pyfunction]
fn pick_next_guess(guess_history: Vec<Word>, result_histories: Vec<Vec<GuessResult>>, n_remaining_words: usize) -> PyResult<String> {
    let next_guess = pick_next_guess_inner(guess_history.as_slice(), result_histories.as_slice(), n_remaining_words)?;
    String::from_utf8(next_guess.bytes().as_slice().to_owned())
        .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Somehow got invalid characters in a word"))
}

/// A Python module implemented in Rust.
#[pymodule]
fn kilordle_droid(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(pick_next_guess, m)?)?;
    Ok(())
}
