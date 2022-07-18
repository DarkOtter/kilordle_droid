import kilordle_droid
import selenium.webdriver
import time

driver = selenium.webdriver.Firefox()
driver.get('https://jonesnxt.github.io/kilordle/')

controller = kilordle_droid.driver.KilordleController(driver)
planner = kilordle_droid.planner.Planner()

while True:
    input("Enter for another guess...")
    print("Finding a guess...")
    next_guess = planner.pick_guess(controller.read_result_history(), controller.remaining_words())
    print("Guessing {}".format(next_guess))
    controller.enter_guess(next_guess)



