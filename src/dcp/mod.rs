use defmt::Format;

#[derive(Format)]
pub enum ServiceType {
    Request = 0,
    Response = 1,
}

impl ServiceType {
    pub fn parse(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Request),
            1 => Some(Self::Response),
            _ => None,
        }
    }
}

#[derive(Format)]
pub enum ServiceID {
    Get = 3,
    Set = 4,
    Identify = 5,
    Hello = 6,
}

impl ServiceID {
    pub fn parse(byte: u8) -> Option<Self> {
        match byte {
            3 => Some(Self::Get),
            4 => Some(Self::Set),
            5 => Some(Self::Identify),
            6 => Some(Self::Hello),
            _ => None,
        }
    }
}

#[derive(Format)]
pub struct DcpPacket<'a> {
    pub frame_id: u16,
    pub service_id: ServiceID,
    pub service_type: ServiceType,
    pub x_id: u32,
    pub response_delay: u16,
    pub length: u32,
    pub payload: &'a [u8],
}
