#![deny(unsafe_code)]
#![no_main]
#![no_std]

use cortex_m_rt::entry;
use microbit::{board::Board, display::blocking::Display, hal::Timer};
use panic_rtt_target as _;
use rtt_target::rtt_init_print;

mod game_of_life;

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);

    let mut display = Display::new(board.display_pins);

    let initial_state_matrix: [[bool; 5]; 5] = [
        [false, false, false, false, false],
        [false, true, true, true, false],
        [true, true, true, false, false],
        [false, false, false, false, false],
        [false, false, false, false, false],
    ];

    let mut state = game_of_life::LifeState {
        matrix: initial_state_matrix,
    };

    loop {
        display.show(&mut timer, state.int_matrix(), 1500);
        state.next_state();
    }
}
