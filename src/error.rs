use defmt::Format;

use crate::{dcp::ParseDcpError, ethernet::EthernetError};

#[derive(Debug, Format)]
pub enum Error {
    DcpError(ParseDcpError),
    EthernetError(EthernetError),
}
