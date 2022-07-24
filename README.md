# kilordle_droid

A very rough bot for [kilordle](https://jonesnxt.github.io/kilordle/) which I wrote to get an idea of how quickly it might be possible to finish the game.

It is packaged as a Python app, which uses selenium to run the web browser with the game, a rust backend for choosing moves,
and in the end also a significant bit of javascript which is used for reading and parsing the game screen (as doing that
in Python was actually the slowest part after the first turn).

The code is not of good quality - I only wrote this as experimental code to answer a question.
