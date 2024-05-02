pub use stm32f4xx_hal::gpio::*;

pub mod frame;
pub mod setup;

pub use frame::*;

pub struct Gpio {
    pub gpioa: gpioa::Parts,
    pub gpiob: gpiob::Parts,
    pub gpioc: gpioc::Parts,
    pub gpiog: gpiog::Parts,
}
