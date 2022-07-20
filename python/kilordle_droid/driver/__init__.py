import importlib.resources
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

from ..util import is_five_lowercase_letters

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


_read_results_div_script = "return " + importlib.resources.read_text('kilordle_droid.driver', 'read_results.js', encoding='utf8')


def _read_results_div(driver: WebDriver, results_elem: WebElement):
    return driver.execute_script(_read_results_div_script, results_elem)



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
        by_column = _read_results_div(self._webdriver, self._results_elem)
        if len(by_column) == 0:
            return [], []
        guess_history = by_column[0]['guessHistory']
        if any(it['guessHistory'] != guess_history for it in by_column):
            raise RuntimeError("Mismatch in guess histories")
        result_histories = [it['resultHistory'] for it in by_column]
        return guess_history, result_histories
