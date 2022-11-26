#![no_main]
#![no_std]

mod game_of_life;
use game_of_life::LifeState;

use core::cell::RefCell;
use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;
use microbit::{
    board::Board,
    display::blocking::Display,
    hal::{gpiote::Gpiote, Timer},
    // The interrupts are imported from the PAC. Since interrupts are chip-specific,
    // they need to be imported from a chip-specific create, such as the PAC (instead of
    // the cortex_m or cortex_m_rt creates).
    pac::{self, interrupt},
};
use panic_rtt_target as _;
use rtt_target::rtt_init_print;

// This Mutex is a wrapper that protects the data inside from being accessed by multiple
// threads at the same time. If one thread wants to access the data inside the Mutex, it
// locks it, preventing other threads from accessing the data until it is done and it
// unlocks it.
// The cortex_m::interrupt Mutex is a special implementation of the Mutex that is safe
// to use when some of the threads that will try to access the data are interrupt
// handler .The way this is done is by implementing the lock of the Mutex in a critical
// section so it can't be interrupted. Otherwise, a deadlock could occur. This is what
// happens when a thread locks the Mutex, and it is interrupted before it can unlock by
// an interrupt that wants to access the Mutex too. The main thread is then halted until
// the interrupt handler is executed, but the interrupt handler is waiting for the main
// thread to unlock the Mutex, causing a permanent locked state.
static GPIO: Mutex<RefCell<Option<Gpiote>>> = Mutex::new(RefCell::new(None));

// Flag that indicates if the game is paused.
static PAUSED: Mutex<RefCell<bool>> = Mutex::new(RefCell::new(false));

// Mutex that contains the game of life state.
static GAME_STATE: Mutex<RefCell<Option<LifeState>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);

    // The GPIO tasks and events (GPIOTE) module provides functionality for accessing
    // GPIO pins using tasks and events. Each GPIOTE channel can be assigned to one pin.
    let gpiote = Gpiote::new(board.GPIOTE);

    // Channel 0 corresponds to the a button.
    let channel0 = gpiote.channel0();
    channel0
        .input_pin(&board.buttons.button_a.degrade())
        .hi_to_lo()
        .enable_interrupt();
    channel0.reset_events();

    // Channel 1 corresponds to the b button.
    let channel1 = gpiote.channel1();
    channel1
        .input_pin(&board.buttons.button_b.degrade())
        .hi_to_lo()
        .enable_interrupt();
    channel1.reset_events();

    // Inside a critical section interrupts are disable. In this case the interrupts
    // are configured inside a critical section to avoid the configuration being
    // interrupted.
    cortex_m::interrupt::free(move |cs| {
        // Processor have a mask that indicate which interrupts are enable and which
        // are not. Masking an interrupt means disabling it, as it is added to the mask,
        // unmasking means enabling it.
        // Unmasking an interrupt is unsafe because it may break critical operations
        // that rely on certain interrupts being masked (disabled).
        unsafe {
            pac::NVIC::unmask(pac::Interrupt::GPIOTE);
        }
        // A pending interupt is an interrupt which has been raised but has not been
        // handled yet by the CPU. The unpend function resets the interrupt pending
        // state.
        pac::NVIC::unpend(pac::Interrupt::GPIOTE);

        // Place the gpiote variable inside GPIO, which is the Mutex that acts as a
        // shared state. Calling the .borrow() method returns the RefCell inside the
        // Mutex, and then calling the .borrow_mut() method returns the Option<Gpiote>
        // inside the RefCell.
        // The cs token needs to be passed to the .borrow() method to ensure it is being
        // called inside a critical section. The contents of a cotex_m::interrupt::Mutex
        // can only be accessed inside a critical section to avoid deadlocks.
        *GPIO.borrow(cs).borrow_mut() = Some(gpiote);
    });

    let mut display = Display::new(board.display_pins);

    let initial_state_matrix: [[bool; 5]; 5] = [
        [false, false, false, false, false],
        [false, true, true, true, false],
        [true, true, true, false, false],
        [false, false, false, false, false],
        [false, false, false, false, false],
    ];

    // Build a LifeState from the matrix and place it into the Mutex.
    cortex_m::interrupt::free(move |cs| {
        *GAME_STATE.borrow(cs).borrow_mut() = Some(LifeState {
            matrix: initial_state_matrix,
        });
    });

    loop {
        // Start a critical section to be able to access the global variables.
        cortex_m::interrupt::free(|cs| {
            if let Some(state) = GAME_STATE.borrow(cs).borrow_mut().as_mut() {
                display.show(&mut timer, state.int_matrix(), 1500);

                // Update the state only if it is not paused. The first call to the
                // .borrow() method is to the Mutex .borrows (this is why it requires
                // the critical section token), which returns a reference to the
                // RefCell. The second call to .borrow() is to the RefCell .borrow()
                // method is to the RefCell .borrow() method, which returns a reference
                // to the boolean inside. This reference is dereferenced using *.
                if !*PAUSED.borrow(cs).borrow() {
                    state.next_state();
                };
            }
        });
    }
}

// Definition of the interrupt handler for the GPIOTE interrupt.
#[interrupt]
fn GPIOTE() {
    // Start a critical section to be able to access the GPIO global variable.
    cortex_m::interrupt::free(|cs| {
        if let Some(gpiote) = GPIO.borrow(cs).borrow().as_ref() {
            let button_a_pressed = gpiote.channel0().is_event_triggered();
            let button_b_pressed = gpiote.channel1().is_event_triggered();

            if button_a_pressed {
                // Replace the boolean value inside PAUSED with its value negated. The
                // .borrow() method returns the RefCell inside the Mutex and the
                // .take() method returns the value inside the RefCell. The .replace()
                // method of the RefCell is then used to substitute the value inside the
                // RefCell (which by now is the default value, since the old one was
                // taken by the .take() method) with the negated (!) from the old value.
                PAUSED.borrow(cs).replace(!PAUSED.borrow(cs).take());
            };
            // Update the state when the button b is pressed and the game is paused.
            if button_b_pressed && *PAUSED.borrow(cs).borrow() {
                GAME_STATE
                    .borrow(cs)
                    .borrow_mut()
                    .as_mut()
                    .unwrap()
                    .next_state();
            };
            // Reset the events.
            gpiote.channel0().reset_events();
            gpiote.channel1().reset_events();
        }
    });
}
