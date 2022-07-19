import kilordle_droid
import selenium.webdriver
import time

webdriver = selenium.webdriver.Firefox()
webdriver.get('https://jonesnxt.github.io/kilordle/')

controller = kilordle_droid.driver.KilordleController(webdriver)

while True:
    input("Enter for another guess...")
    print("Finding a guess...")
    guess_history, result_histories = controller.read_result_history()
    next_guess = kilordle_droid.pick_next_guess(guess_history, result_histories, controller.remaining_words())
    print("Guessing {}".format(next_guess))
    controller.enter_guess(next_guess)



