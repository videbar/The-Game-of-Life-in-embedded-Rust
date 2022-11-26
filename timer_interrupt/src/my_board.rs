use microbit::{
    gpio::DisplayPins,
    hal::gpio::{
        p0::{Parts, P0_14, P0_23},
        p1, Floating, Input, Level,
    },
    pac::{Peripherals, CLOCK, RTC0, RTC1, TIMER0},
};

// A struc that represents the microbit board and contains the peripherals that are
// relevant for this project.
pub struct MyBoard {
    // Pins that drive the 5x5 LED matrix:
    pub display_pins: DisplayPins,
    // Buttons in the board:
    pub button_a: P0_14<Input<Floating>>,
    pub button_b: P0_23<Input<Floating>>,
    // Two of the real time counters:
    pub rtc0: RTC0,
    pub rtc1: RTC1,
    // One of the timers:
    pub timer0: TIMER0,
    // The clock:
    pub clock: CLOCK,
}

impl MyBoard {
    // Returns an instance of MyBoard only if it's the first time the method is called.
    // This is done to avoid having two variables that control the same hardware
    // components.
    pub fn take() -> Option<Self> {
        match Peripherals::take() {
            Some(peripherals) => {
                let p0_parts = Parts::new(peripherals.P0);
                let p1_parts = p1::Parts::new(peripherals.P1);
                Some(Self {
                    display_pins: DisplayPins {
                        col1: p0_parts.p0_28.into_push_pull_output(Level::High),
                        col2: p0_parts.p0_11.into_push_pull_output(Level::High),
                        col3: p0_parts.p0_31.into_push_pull_output(Level::High),
                        col4: p1_parts.p1_05.into_push_pull_output(Level::High),
                        col5: p0_parts.p0_30.into_push_pull_output(Level::High),
                        row1: p0_parts.p0_21.into_push_pull_output(Level::Low),
                        row2: p0_parts.p0_22.into_push_pull_output(Level::Low),
                        row3: p0_parts.p0_15.into_push_pull_output(Level::Low),
                        row4: p0_parts.p0_24.into_push_pull_output(Level::Low),
                        row5: p0_parts.p0_19.into_push_pull_output(Level::Low),
                    },
                    button_a: p0_parts.p0_14.into_floating_input(),
                    button_b: p0_parts.p0_23.into_floating_input(),
                    rtc0: peripherals.RTC0,
                    rtc1: peripherals.RTC1,
                    timer0: peripherals.TIMER0,
                    clock: peripherals.CLOCK,
                })
            }
            None => None,
        }
    }
}
