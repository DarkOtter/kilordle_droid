from typing import List

from . import kilordle_droid as _kilordle_droid_rs
from . import driver


def pick_next_guess(guess_history: List[str], result_histories: List[List[str]], n_remaining_words: int) -> str:
	return _kilordle_droid_rs.pick_next_guess(guess_history, result_histories, n_remaining_words)