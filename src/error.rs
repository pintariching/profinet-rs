use crate::{dcp::ParseDcpError, ethernet::EthernetError};

pub enum Error {
    DcpError(ParseDcpError),
    EthernetError(EthernetError),
}
