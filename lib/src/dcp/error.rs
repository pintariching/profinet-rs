#[derive(Debug)]
pub enum ParseDCPError {
    HeaderError(ParseDCPHeaderError),
    BlockError(ParseDCPBlockError),
}

#[derive(Debug)]
pub enum ParseDCPHeaderError {
    InvalidHeaderLength,
    InvalidFrameID,
    InvalidServiceID,
    InvalidServiceType,
}

#[derive(Debug)]
pub enum ParseDCPBlockError {
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
