#[derive(Debug)]
pub enum ParseDcpError {
    FrameIdError,
    HeaderError(ParseDcpHeaderError),
    BlockError(ParseDcpBlockError),
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
