import kilordle_droid
import selenium.webdriver
import time

webdriver = selenium.webdriver.Firefox()
webdriver.get('https://jonesnxt.github.io/kilordle/')

controller = kilordle_droid.driver.KilordleController(webdriver)

last_enter = time.time()
time_between_guesses = 0.25
i = 0
while True:
    time.sleep(0.01)
    words_remaining = controller.remaining_words()
    if words_remaining == 0:
        break
    time_before_read = time.time()
    guess_history, result_histories = controller.read_result_history()
    time_before_find_guess = time.time()
    next_guess = kilordle_droid.pick_next_guess(guess_history, result_histories, controller.remaining_words())
    time_after_find_guess = time.time()
    print("Found guess {} in {:.2f}s (took {:.2f}s to read screen)".format(next_guess, time_after_find_guess - time_before_find_guess, time_before_find_guess - time_before_read))
    next_enter = last_enter + time_between_guesses
    if time_after_find_guess < next_enter:
        last_enter = next_enter
        time.sleep(next_enter - time_after_find_guess)
    else:
        last_enter = time_after_find_guess
    controller.enter_guess(next_guess)
    i += 1

input('Waiting before closing...')
webdriver.quit()
exit(0)

