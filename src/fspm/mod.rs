pub mod app;
mod configuration;

use app::*;
use configuration::*;

use crate::{
    constants::{MAX_PHYSICAL_PORTS, MAX_PRODUCT_NAME_SIZE, MAX_STATION_NAME_SIZE},
    scheduler::TaskCallback,
    PNet,
};

#[derive(Clone)]
pub struct Config<T: App> {
    /// Tick interval in microseconds
    pub tick_us: usize,
    pub app: T,

    pub im0: IM0,
    pub im1: IM1,
    pub im2: IM2,
    pub im3: IM3,
    pub im4: IM4,

    pub device_id: DeviceIdConfig,
    pub oem_device_id: DeviceIdConfig,

    pub station_name: [u8; MAX_STATION_NAME_SIZE],
    pub product_name: [u8; MAX_PRODUCT_NAME_SIZE],

    pub min_data_exchange_interval: usize,
    pub send_dcp_hello: bool,

    pub num_physical_ports: usize,
    pub use_qualified_diagnosis: bool,
    pub interface_config: InterfaceConfig,
}

impl<T> Config<T>
where
    T: App + Copy,
{
    pub fn init<U: TaskCallback + Copy>(mut self, pnet: &mut PNet<T, U>) {
        self.validate_config();
        pnet.fspm_default_config = self.clone();

        self.app.signal_led_ind(pnet, false);
        pnet.fspm_user_config = self;
    }

    fn validate_config(&self) {
        let im_mask = 2 | 4 | 8 | 16;

        if self.tick_us == 0 {
            defmt::panic!("Tick interval must be more than 0.");
        }

        if self.interface_config.network_interface_name.is_empty() {
            defmt::panic!("Network interface must have a name");
        }

        if self.num_physical_ports == 0 || self.num_physical_ports > MAX_PHYSICAL_PORTS {
            defmt::panic!(
                "Wrong number of physical ports. Got {}, must be between 1 and {}",
                self.num_physical_ports,
                MAX_PHYSICAL_PORTS
            );
        }

        if self.min_data_exchange_interval == 0 {
            defmt::panic!("min_data_exchange_interval must be more than 0");
        }

        if self.min_data_exchange_interval > 4096 {
            defmt::panic!("min_data_exchange_interval is too large");
        }

        if (self.im0.supported & im_mask) > 0 {
            defmt::panic!(
                "I&M supported setting is wrong. Got {}, must be {}",
                self.im0.supported,
                im_mask
            );
        }
    }
}
