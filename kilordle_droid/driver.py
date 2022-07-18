import re
import time
from typing import Union, List, Literal, Tuple

from selenium.webdriver.common.by import By
from selenium.webdriver.common.keys import Keys
from selenium.webdriver.remote.webdriver import WebDriver
from selenium.webdriver.remote.webelement import WebElement
from selenium.webdriver.support import expected_conditions as EC
from selenium.webdriver.support.color import Color
from selenium.webdriver.support.wait import WebDriverWait

from kilordle_droid.util import is_five_lowercase_letters

DriverOrElement = Union[WebDriver, WebElement]

_rex_whitespace = re.compile(r'\s+')


def _remove_all_whitespace(string: str) -> str:
    return _rex_whitespace.sub('', string)


def _parent_element(elem: WebElement) -> WebElement:
    return elem.find_element(By.XPATH, './..')


def _child_divs(elem: WebElement) -> List[WebElement]:
    return elem.find_elements(By.XPATH, './div')


def _remove_ancestors(elems: List[WebElement]) -> List[WebElement]:
    """Return a new list with only elements that are not ancestors of
    other elements in the list."""
    result = elems[:]
    for elem in elems:
        try:
            result.remove(_parent_element(elem))
        except ValueError:
            pass
    return result


def _find_navbar_elem_or_false(root: DriverOrElement) -> Union[WebElement, Literal[False]]:
    potentials = _remove_ancestors([
        elem for elem in root.find_elements(By.CSS_SELECTOR, 'div.App div')
        if 'Kilordle' in elem.text and 'Remaining:' in elem.text ])
    if len(potentials) != 1:
        return False
    return potentials[0]


def _following_sibling(elem: WebElement) -> WebElement:
    return elem.find_element(By.XPATH, './following-sibling::*[1]')

def _rows_of_result_history(elem: WebElement) -> List[WebElement]:
    all_rows = _child_divs(elem)
    if len(all_rows) < 1:
        raise ValueError("Result history element must have at least one child div")
    if all_rows[-1].text.strip() != '':
        raise ValueError("Result history element must have last row blank")
    del all_rows[-1]
    return all_rows


class KilordleController:
    _webdriver: WebDriver
    _body_elem: WebElement
    _navbar_elem: WebElement
    _results_elem: WebElement
    _remaining_rex: re.Pattern

    def __init__(self, webdriver: WebDriver):
        self._webdriver = webdriver
        self._body_elem = WebDriverWait(webdriver, 10).until(
            EC.presence_of_element_located((By.CSS_SELECTOR, 'body')))
        self._navbar_elem = WebDriverWait(webdriver, 10).until(_find_navbar_elem_or_false)
        self._results_elem = _following_sibling(self._navbar_elem)
        self._remaining_rex = re.compile(r'Remaining: (\d*)/1000')

    def enter_guess(self, guess: str):
        guess = guess.lower()
        if not is_five_lowercase_letters(guess):
            raise ValueError("Guess must be exactly 5 letters")
        self._body_elem.send_keys(guess)
        time.sleep(0.1)
        self._body_elem.send_keys(Keys.ENTER)

    def remaining_words(self) -> int:
        return int(self._remaining_rex.search(self._navbar_elem.text).group(1))

    def read_result_history(self) -> Tuple[List[str], List[List[str]]]:
        result_elems = [
            elem for elem in _child_divs(self._results_elem)
            if '+' not in elem.text and len(_child_divs(elem)) >= 1]
        if len(result_elems) < 1:
            return [], []

        guess_history = []
        for row_elem in _rows_of_result_history(result_elems[0]):
            guess = _remove_all_whitespace(row_elem.text)
            if not is_five_lowercase_letters(guess):
                raise RuntimeError("Result history has invalid guess: {}".format(repr(guess)))
            guess_history.append(guess)

        result_histories = []
        for result_elem in result_elems:
            result_history = []
            for i, row_elem in enumerate(_rows_of_result_history(result_elem)):
                guess = ''
                result = ''
                for letter_elem in row_elem.find_elements(By.XPATH, './div'):
                    letter = letter_elem.text.strip()
                    if not ('a' <= letter <= 'z'):
                        raise RuntimeError('Expected every guess result row letter to have a letter as its text')
                    guess += letter
                    elem_colour = Color.from_string(letter_elem.value_of_css_property('background-color'))
                    if elem_colour.green < 50:
                        raise RuntimeError('Unexpected colour of letter element')
                    elif elem_colour.red < 50:
                        result += 'O'  # Full match
                    elif elem_colour.blue < 50:
                        result += 'o'  # Elsewhere match
                    else:
                        result += ' '
                if not is_five_lowercase_letters(guess):
                    raise RuntimeError('Expected every guess result row to have 5 lowercase letters')
                elif guess_history[i] != guess:
                    raise RuntimeError('Saw a result with a guess mistmatch')
                result_history.append(result)
            if len(result_history) >= 1:
                result_histories.append(result_history)

        return guess_history, result_histories
