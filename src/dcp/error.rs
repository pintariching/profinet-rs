use nom::error::{ErrorKind, ParseError};

#[derive(Debug)]
pub enum ParseDcpError<I> {
    HeaderError(ParseDcpHeaderError),
    BlockError(ParseDcpBlockError),
    Nom(I, ErrorKind),
}

#[derive(Debug)]
pub enum ParseDcpHeaderError {
    InvalidHeaderLength,
    InvalidFrameID,
    InvalidServiceID,
    InvalidServiceType,
}

#[derive(Debug)]
pub enum ParseDcpBlockError {
    InvalidBlockOption,
    InvalidIPSuboption,
    InvalidDevicePropertySuboption,
    InvalidDevicePropertyBlock,
    InvalidDHCPPropertySuboption,
    InvalidControlSuboption,
    InvalidDeviceInitiativeSuboption,
    InvalidNMEDomainSuboption,
    InvalidManufacturerSpecificSuboption,
    InvalidAllSuboption,
    InvalidDeviceRole,
}

impl<I> ParseError<I> for ParseDcpError<I> {
    fn from_error_kind(input: I, kind: nom::error::ErrorKind) -> Self {
        Self::Nom(input, kind)
    }

    fn append(_: I, _: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}
