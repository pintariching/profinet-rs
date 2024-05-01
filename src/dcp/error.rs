use defmt::Format;

#[derive(Debug, Format)]
pub enum ParseDcpError {
    FrameIdError,
    HeaderError(ParseDcpHeaderError),
    BlockError(ParseDcpBlockError),
}

#[derive(Debug, Format)]
pub enum ParseDcpHeaderError {
    InvalidHeaderLength,
    InvalidFrameID,
    InvalidServiceID,
    InvalidServiceType,
}

#[derive(Debug, Format)]
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
    InvalidIpParameterBlockInfo,
}
