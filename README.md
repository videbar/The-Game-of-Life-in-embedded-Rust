# The Game of Life in embedded Rust

This repository contains an implementation of [Conways's game of Life](
https://en.wikipedia.org/wiki/Conway's_Game_of_Life) on a [Microbit board](
https://tech.microbit.org/hardware/) using embedded Rust. It was a learning project
that I used to get familiar with certain aspects of embedded rust programming, such as
concurrency, interrupts, and global mutable variables.

The initial state of the game can be defined and it will be shown on the board 5x5
LED matrix. It will be periodically updated following the game rules and the evolution
can be paused and resumed with the A button. While the evolution is halted, the B
button can be used to jump directly to the next generation.

I implemented this idea in two different ways. At first, I used GPIO interrupts to
catch the button presses and I drove the LED display inside the `loop {}`. This
first version can be found in the `gpio_interrupt` directory.

I found this approach unreliable due to switch bouncing, so I developed a second
version in which I used timers (in particular I used the real time counters of the
microcontroller) to poll the state of the buttons and dictate the evolution of the
game. This second version can be found on the `timer_interrupt` directory.

Since this was a learning project, all the code is heavily commented, and you can
find more information [on my blog](https://vide.bar/blog/rust-microbit-game-of-life).
