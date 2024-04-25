use crate::{dcp::ParseDcpError, ethernet::error::EthernetError};

pub enum Error<I> {
    DcpError(ParseDcpError<I>),
    EthernetError(EthernetError),
}
