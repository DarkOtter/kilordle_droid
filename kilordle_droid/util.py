import re

_rex_is_five_lowercase_letters = re.compile(r'[a-z]{5}')


def is_five_lowercase_letters(guess: str) -> bool:
    return bool(_rex_is_five_lowercase_letters.match(guess))
