use stm32_eth::{hal::gpio::*, EthPins};

use super::Gpio;

pub fn setup_pins(
    gpio: Gpio,
) -> EthPins<PA1<Input>, PA7<Input>, PG11<Input>, PG13<Input>, PB13<Input>, PC4<Input>, PC5<Input>>
{
    let Gpio {
        gpioa,
        gpiob,
        gpioc,
        gpiog,
    } = gpio;

    let ref_clk = gpioa.pa1.into_floating_input();
    let crs = gpioa.pa7.into_floating_input();
    let tx_d1 = gpiob.pb13.into_floating_input();
    let rx_d0 = gpioc.pc4.into_floating_input();
    let rx_d1 = gpioc.pc5.into_floating_input();

    let (tx_en, tx_d0) = {
        (
            gpiog.pg11.into_floating_input(),
            gpiog.pg13.into_floating_input(),
        )
    };

    EthPins {
        ref_clk,
        crs,
        tx_en,
        tx_d0,
        tx_d1,
        rx_d0,
        rx_d1,
    }
}
