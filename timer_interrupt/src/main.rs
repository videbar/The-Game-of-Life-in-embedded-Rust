#![no_main]
#![no_std]

mod game_of_life;
use game_of_life::LifeState;

mod my_board;
use my_board::MyBoard;

use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;
use microbit::{
    display::nonblocking::{BitImage, Display},
    hal::{
        clocks::Clocks,
        gpio::{
            p0::{P0_14, P0_23},
            Floating, Input,
        },
        prelude::InputPin,
        rtc::{Rtc, RtcCompareReg, RtcInterrupt},
    },
    // The interrupts are imported from the PAC. Since interrupts are chip-specific,
    // they need to be imported from a chip-specific create, such as the PAC (instead of
    // the cortex_m or cortex_m_rt creates).
    pac::{self, interrupt, RTC0, RTC1, TIMER0},
};
use panic_rtt_target as _;
use rtt_target::rtt_init_print;

// These Mutex are a wrapper that protects the data inside from being accessed by
// multiple threads at the same time. If one thread wants to access the data inside the
// Mutex, it locks it, preventing other threads from accessing the data until it is done
// and it unlocks it.
// The cortex_m::interrupt Mutex is a special implementation of the Mutex that is safe
// to use when some of the threads that will try to access the data are interrupt
// handlers. The way this is done is by implementing the lock of the Mutex in a critical
// section so it can't be interrupted. Otherwise, a deadlock could occur. This is what
// happens when a thread locks the Mutex, and it is interrupted before it can unlock by
// an interrupt that wants to access the Mutex too. The main thread is then halted until
// the interrupt handler is executed, but the interrupt handler is waiting for the main
// thread to unlock the Mutex, causing a permanent locked state.
// The RefCell inside the Mutex are also a data wrapper, in this case they provide
// interior mutability. This means that the data inside the RefCell can be mutated,
// even though the RefCell itself is not mutable.
// By combining a Mutex and a RefCell, it's possible to define global mutable variables,
// i.e., variables that can be accessed from various threads (thanks to the Mutex) and
// can be modified (thanks to the RefCell and interior mutability).
// If the initial value of the global mutable value is not known yet, an additional
// Option can be placed inside the RefCell. The None variant acts then as a placeholder
// until a value is placed in the RefCell.

// Real-time counter that is used to poll the state of the buttons.
static BUTTON_COUNTER: Mutex<RefCell<Option<Rtc<RTC0>>>> = Mutex::new(RefCell::new(None));
// Real-time counter that is used to update the figure shown in the display.
static DISPLAY_COUNTER: Mutex<RefCell<Option<Rtc<RTC1>>>> = Mutex::new(RefCell::new(None));

// Button a, used to pause/resume the game.
static BUTTON_A: Mutex<RefCell<Option<P0_14<Input<Floating>>>>> = Mutex::new(RefCell::new(None));
// Flag to kep track of the previous state of the button.
static BUTTON_A_WAS_PRESSED: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));

// Button b, used to update the state of the game if the game is paused.
static BUTTON_B: Mutex<RefCell<Option<P0_23<Input<Floating>>>>> = Mutex::new(RefCell::new(None));
// Flag to kep track of the previous state of the button.
static BUTTON_B_WAS_PRESSED: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));

static DISPLAY: Mutex<RefCell<Option<Display<TIMER0>>>> = Mutex::new(RefCell::new(None));
static GAME_STATE: Mutex<RefCell<Option<LifeState>>> = Mutex::new(RefCell::new(None));
// Flag to keep track of whether or not the game is paused.
static PAUSED: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = MyBoard::take().unwrap();

    // Starting the low-frequency clock. This is needed for the real timer counters.
    Clocks::new(board.clock).start_lfclk();

    // Create a new display. The timer0 of the board is used to drive the display.
    let display = Display::new(board.timer0, board.display_pins);

    // Create and configure the real time counter (RTCs). The rtc0 is used to
    // periodically poll the buttons to check if they have been pressed and the rtc1 is
    // used to update the game state shown on the display. The frequency of the RTCs is
    // given by: f [Hz] = 32768 / (prescaler + 1 ).

    // The counter used to poll the buttons has a frequency of 166.66 Hz and a period
    // of approximately 6ms.
    let mut button_counter = Rtc::new(board.rtc0, 196).unwrap();
    button_counter.enable_event(RtcInterrupt::Tick);
    button_counter.enable_interrupt(RtcInterrupt::Tick, None);
    button_counter.enable_counter();

    // The counter used to update the display has a frequency of 8 Hz and a period of
    // 125 ms. This is maximum value for the period. The Compare value is set to 8,
    // which means that Compare0 interrupt will be called after 8 periods of time, i.e.,
    // after 1 second.
    let mut display_counter = Rtc::new(board.rtc1, 4095).unwrap();
    display_counter
        .set_compare(RtcCompareReg::Compare0, 8)
        .unwrap();
    display_counter.enable_event(RtcInterrupt::Compare0);
    display_counter.enable_interrupt(RtcInterrupt::Compare0, None);
    display_counter.enable_counter();

    // Set the initial state of the game of life.
    let initial_state_matrix: [[bool; 5]; 5] = [
        [false, false, false, false, false],
        [false, true, true, true, false],
        [true, true, true, false, false],
        [false, false, false, false, false],
        [false, false, false, false, false],
    ];
    let initial_state = LifeState {
        matrix: initial_state_matrix,
    };

    // Inside a critical section interrupts are disable. In this case the interrupts
    // are configured inside a critical section to avoid the configuration being
    // interrupted.
    cortex_m::interrupt::free(move |cs| {
        // Processors have a mask that indicate which interrupts are enable and which
        // are not. Masking an interrupt means disabling it, as it is added to the mask,
        // unmasking means enabling it.
        // Unmasking an interrupt is unsafe because it may break critical operations
        // that rely on certain interrupts being masked (disabled).
        unsafe {
            pac::NVIC::unmask(pac::Interrupt::RTC0);
            pac::NVIC::unmask(pac::Interrupt::RTC1);
            pac::NVIC::unmask(pac::Interrupt::TIMER0);
        }

        // A pending interupt is an interrupt which has been raised but has not been
        // handled yet by the CPU. The unpend function resets the interrupt pending
        // state.
        pac::NVIC::unpend(pac::Interrupt::RTC0);
        pac::NVIC::unpend(pac::Interrupt::RTC1);
        pac::NVIC::unpend(pac::Interrupt::TIMER0);

        // Place the values inside the Mutex that acts as a shared state. Calling the
        // .borrow() method returns the RefCell inside the Mutex, and then calling the
        // .replace() method allows to replace the value inside the RefCell.
        // The cs token needs to be passed to the .borrow() method to ensure it is being
        // called inside a critical section. The contents of a cotex_m::interrupt::Mutex
        // can only be accessed inside a critical section to avoid deadlocks.

        BUTTON_COUNTER.borrow(cs).replace(Some(button_counter));
        DISPLAY_COUNTER.borrow(cs).replace(Some(display_counter));

        BUTTON_A.borrow(cs).replace(Some(board.button_a));
        BUTTON_B.borrow(cs).replace(Some(board.button_b));

        DISPLAY.borrow(cs).replace(Some(display));
        GAME_STATE.borrow(cs).replace(Some(initial_state))
    });

    loop {}
}

// This interrupt is used to drive the display. It takes care of updating the LED
// display and clearing the timer's event registers.
#[interrupt]
fn TIMER0() {
    cortex_m::interrupt::free(|cs| {
        if let Some(display) = DISPLAY.borrow(cs).borrow_mut().as_mut() {
            display.handle_display_event();
        };
    });
}

// Interrupt used to poll the buttons. It will be called approximately every 6ms.
#[interrupt]
fn RTC0() {
    cortex_m::interrupt::free(move |cs| {
        if let Some(button_a) = BUTTON_A.borrow(cs).borrow().as_ref() {
            if let Ok(a_pressed) = button_a.is_low() {
                // Check if the button a is being pressed.
                if a_pressed {
                    // The game should be paused/ resumed only if the buttons a is
                    // being pressed and was not being pressed before, this is, the
                    // game is only paused/resumed on the press on the button, but it's
                    // not being constantly paused/resumed while the buttons is kept
                    // pressed. The global mutable variable BUTTON_A_WAS_PRESSED is used
                    // to keep track of the previous state of the button.
                    // The .replace() method does two things. First it replaces the old
                    // value contained in BUTTON_WAS_PRESSED with true, since the button
                    // is now being pressed. Second, it returns the old value contained
                    // in BUTTON_WAS_PRESSED, which is used to check if the button has
                    // just been pressed.
                    if !BUTTON_A_WAS_PRESSED.borrow(cs).replace(true) {
                        // If the button has just been pressed, the value inside PAUSED
                        // is negated.
                        PAUSED.borrow(cs).replace_with(|&mut old_value| !old_value);
                    };
                } else {
                    // Finally, if the button is not being pressed, the value inside
                    // BUTTON_A_WAS_PRESSED is set to false.
                    BUTTON_A_WAS_PRESSED.borrow(cs).replace(false);
                };
            };
        };
        if let Some(button_b) = BUTTON_B.borrow(cs).borrow().as_ref() {
            if let Ok(b_pressed) = button_b.is_low() {
                if b_pressed {
                    // The same logic is followed as for the button a.
                    if !BUTTON_B_WAS_PRESSED.borrow(cs).replace(true) {
                        // Button b will update the game state shown on the screen only
                        // if the game is paused.
                        if *PAUSED.borrow(cs).borrow() {
                            if let Some(game_state) = GAME_STATE.borrow(cs).borrow_mut().as_mut() {
                                game_state.next_state();
                                if let Some(display) = DISPLAY.borrow(cs).borrow_mut().as_mut() {
                                    let image = BitImage::new(&game_state.int_matrix());
                                    display.show(&image);
                                };
                            }
                        }
                    };
                } else {
                    BUTTON_B_WAS_PRESSED.borrow(cs).replace(false);
                };
            };
        };

        if let Some(button_counter) = BUTTON_COUNTER.borrow(cs).borrow_mut().as_mut() {
            button_counter.reset_event(RtcInterrupt::Tick);
        }
    });
}

// Interrupt used to update the display. It will be called approximately every second.
#[interrupt]
fn RTC1() {
    cortex_m::interrupt::free(move |cs| {
        if let Some(display) = DISPLAY.borrow(cs).borrow_mut().as_mut() {
            if let Some(game_state) = GAME_STATE.borrow(cs).borrow_mut().as_mut() {
                if !*PAUSED.borrow(cs).borrow() {
                    game_state.next_state();
                    let image = BitImage::new(&game_state.int_matrix());
                    display.show(&image);
                }
            }
        };

        if let Some(display_counter) = DISPLAY_COUNTER.borrow(cs).borrow_mut().as_mut() {
            display_counter.reset_event(RtcInterrupt::Compare0);
            // This interrupt uses a counter. A the value in the counter is incremented
            // by one with the frequency of the RTC, in this case every 125 ms. When
            // the counter reaches the value in the compare register, in this case 8,
            // the interrupt is called, in this case after 1 second. When this happens
            // the counter must be cleared so that it starts counting from 0 again.
            display_counter.clear_counter();
        };
    });
}
