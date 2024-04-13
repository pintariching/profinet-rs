#[derive(Debug)]
pub enum ParseDCPHeaderError {
    InvalidHeaderLength,
    InvalidFrameID,
    InvalidServiceID,
    InvalidServiceType,
}

#[derive(Debug)]
pub enum ParseDCPError {
    HeaderError(ParseDCPHeaderError),
}
