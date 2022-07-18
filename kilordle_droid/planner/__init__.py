import importlib.resources
import itertools
from dataclasses import dataclass
from gzip import GzipFile
from io import TextIOWrapper
from typing import List, Tuple, Optional
from .. import util
import numpy as np




def _load_wordlist(resource_name: str) -> List[str]:
    with importlib.resources.open_binary('kilordle_droid.planner', resource_name) as read_raw:
        with GzipFile(fileobj=read_raw, mode='r') as read_gzip:
            with TextIOWrapper(read_gzip, encoding='ascii') as read_text:
                result = []
                for line in read_text:
                    line = line.strip()
                    if line == '':
                        continue
                    if not util.is_five_lowercase_letters(line):
                        raise RuntimeError('Wordlist has an invalid word: {}'.format(repr(line)))
                    result.append(line)
                return result


def _is_possible_result(guess: str, result: str, word: str) -> bool:
    remove_at = []
    partial_matches = []
    mismatches = []
    word_remaining = list(word)
    for i, (c, m) in enumerate(zip(guess, result)):
        if m == 'O':
            if word[i] != c:
                return False
            remove_at.append(i)
        elif m == 'o':
            partial_matches.append(c)
        elif m == ' ':
            mismatches.append(c)
    while remove_at:
        del word_remaining[remove_at.pop()]
    for c in partial_matches:
        if c not in word_remaining:
            return False
        word_remaining.remove(c)
    return all(c not in word_remaining for c in mismatches)


def _score_in_history(guess_history: List[str], word: str) -> int:
    total_score = 0
    for i, c in enumerate(word):
        position_score = 0
        for guess in guess_history:
            if guess[i] == c:
                position_score = 3
                break
            elif c in guess:
                position_score = 1
        total_score += position_score
    return total_score

def _is_possible_result_history(guess_history: List[str], result_history: List[str], word: str) -> bool:
    if not util.is_five_lowercase_letters(word):
        raise ValueError("Word would not be a valid guess")
    if len(guess_history) != len(result_history):
        raise ValueError("Guess and result history must be the same length")
    for guess, result in zip(guess_history, result_history):
        if not util.is_five_lowercase_letters(guess):
            raise ValueError("History contains an invalid guess")
        if not _is_possible_result(guess, result, word):
            return False
    else:
        return True


def _average_score(guess_history: List[str], words: List[str]) -> float:
    return np.mean(np.fromiter((_score_in_history(guess_history, word) for word in words), dtype='float64', count=len(words)))


class GuessEvaluator:
    possible_words_for_histories: List[List[str]]
    possible_words_for_pile: List[str]
    n_for_pile: int

    def __init__(self):
        raise RuntimeError("You should not init this class directly")

    @classmethod
    def prep_calculation(cls, all_wordles: List[str], guess_history: List[str], result_histories: List[List[str]], n_words_remaining: int) -> 'GuessEvaluator':
        if n_words_remaining < len(result_histories):
            raise ValueError('The number of words remaining cannot be less than the number of histories')
        if n_words_remaining < 1:
            raise ValueError('There are no wordles left to guess')

        result = object.__new__(cls)

        words_with_score = []
        for word in all_wordles:
            score = _score_in_history(guess_history, word)
            if score >= 15:
                continue
            words_with_score.append((word, score))

        words_for_histories = []
        max_scores_for_histories = []
        for result_history in result_histories:
            max_score = 0
            words_for_history = []
            for word, score in words_with_score:
                if not _is_possible_result_history(guess_history, result_history, word):
                    continue
                max_score = max(max_score, score)
                words_for_history.append(word)

            words_for_histories.append(words_for_history)
            max_scores_for_histories.append(max_score)

        max_score_for_pile = min(max_scores_for_histories) if len(max_scores_for_histories) >= 1 else 15 - 1
        words_for_pile = [word for word, score in words_with_score if score <= max_score_for_pile]
        del words_with_score

        result.possible_words_for_histories = words_for_histories
        result.possible_words_for_pile = words_for_pile
        result.n_for_pile = n_words_remaining - len(words_for_histories) # This is probably not exact, but a guess
        return result

    def score_guess_history(self, guess_history: List[str]) -> float:
        total_score = 0
        for x in self.possible_words_for_histories:
            total_score += _average_score(guess_history, x)
        total_score += self.n_for_pile * _average_score(guess_history, self.possible_words_for_pile)
        return total_score


class Planner:
    _all_wordles: List[str]
    _all_other_words: List[str]

    def __init__(self):
        self._all_wordles = _load_wordlist('wordles.txt.gz')
        self._all_other_words = _load_wordlist('other_words.txt.gz')

    def pick_guess(self, result_history: Tuple[List[str], List[List[str]]], remaining_words: int) -> str:
        guess_history, result_histories = result_history
        evaluator = GuessEvaluator.prep_calculation(self._all_wordles, guess_history, result_histories, remaining_words)

        best_score = 0
        best_word = 'deair'
        for word in itertools.chain(self._all_wordles, self._all_other_words):
            guess_history.append(word)
            score = evaluator.score_guess_history(guess_history)
            if score > best_score:
                best_score = score
                best_word = word
            guess_history.pop()
        return best_word