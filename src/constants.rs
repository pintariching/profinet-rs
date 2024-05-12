pub const MAX_AR: usize = 1;
pub const MAX_CR: usize = 2;
pub const MAX_PHYSICAL_PORTS: usize = 1;
pub const MAX_SCHEDULER_TASKS: usize = 2 * (MAX_AR) * (MAX_CR) + 2 * (MAX_PHYSICAL_PORTS) + 9;

pub const MAX_ORDER_ID_LENGTH: usize = 20;
pub const MAX_SERIAL_NUMBER_LENGTH: usize = 16;
pub const MAX_LOCATION_SIZE: usize = 22;
pub const MAX_STATION_NAME_SIZE: usize = 240;
pub const MAX_PRODUCT_NAME_SIZE: usize = 25;
