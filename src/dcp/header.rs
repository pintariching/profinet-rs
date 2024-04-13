use byteorder::{ByteOrder, NetworkEndian};
use num_enum::TryFromPrimitive;

use super::error::ParseDCPHeaderError;

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(TryFromPrimitive)]
#[repr(u16)]
pub enum FrameID {
    Hello = 0xfefc,
    GetSet = 0xfefd,
    Request = 0xfefe,
    Reset = 0xfeff,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum ServiceType {
    Request = 0,
    Success = 1,
    NotSupported = 5,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum ServiceID {
    Get = 3,
    Set = 4,
    Identify = 5,
    Hello = 6,
}

mod header_field {
    use crate::field::*;

    pub const FRAME_ID: Field = 0..2;
    pub const SERVICE_ID: SmallField = 2;
    pub const SERVICE_TYPE: SmallField = 3;
    pub const X_ID: Field = 4..8;
    pub const RESPONSE_DELAY: Field = 8..10;
    pub const DATA_LENGTH: Field = 10..12;
    pub const PAYLOAD: Rest = 12..;
}

pub const DCP_HEADER_LENGTH: usize = header_field::PAYLOAD.start;

pub struct DCPHeaderFrame<T: AsRef<[u8]>> {
    buffer: T,
}

impl<T: AsRef<[u8]>> DCPHeaderFrame<T> {
    pub const fn new_unchecked(buffer: T) -> Self {
        DCPHeaderFrame { buffer }
    }

    pub fn new_checked(buffer: T) -> Result<Self, ParseDCPHeaderError> {
        let header = Self::new_unchecked(buffer);
        match header.check_len() {
            true => Ok(header),
            false => Err(ParseDCPHeaderError::InvalidHeaderLength),
        }
    }

    pub fn check_len(&self) -> bool {
        let len = self.buffer.as_ref().len();

        len > DCP_HEADER_LENGTH
    }

    pub fn frame_id(&self) -> Result<FrameID, ParseDCPHeaderError> {
        let data = self.buffer.as_ref();
        let raw = NetworkEndian::read_u16(&data[header_field::FRAME_ID]);
        FrameID::try_from_primitive(raw).map_err(|_| ParseDCPHeaderError::InvalidFrameID)
    }

    pub fn service_id(&self) -> Result<ServiceID, ParseDCPHeaderError> {
        let data = self.buffer.as_ref();
        let raw = data[header_field::SERVICE_ID];
        ServiceID::try_from_primitive(raw).map_err(|_| ParseDCPHeaderError::InvalidServiceID)
    }

    pub fn service_type(&self) -> Result<ServiceType, ParseDCPHeaderError> {
        let data = self.buffer.as_ref();
        let raw = data[header_field::SERVICE_TYPE];
        ServiceType::try_from_primitive(raw).map_err(|e| ParseDCPHeaderError::InvalidServiceType)
    }

    pub fn x_id(&self) -> u32 {
        let data = self.buffer.as_ref();
        NetworkEndian::read_u32(&data[header_field::X_ID])
    }

    pub fn response_delay(&self) -> u16 {
        let data = self.buffer.as_ref();
        NetworkEndian::read_u16(&data[header_field::RESPONSE_DELAY])
    }

    pub fn data_length(&self) -> u16 {
        let data = self.buffer.as_ref();
        NetworkEndian::read_u16(&data[header_field::DATA_LENGTH])
    }

    pub fn payload(&self) -> &[u8] {
        let data = self.buffer.as_ref();
        &data[header_field::PAYLOAD]
    }
}

pub struct DCPHeader {
    pub frame_id: FrameID,
    pub service_id: ServiceID,
    pub service_type: ServiceType,
    pub x_id: u32,
    pub response_delay: u16,
    pub data_length: u16,
}

impl DCPHeader {
    pub fn parse<T: AsRef<[u8]>>(frame: &DCPHeaderFrame<T>) -> Result<Self, ParseDCPHeaderError> {
        Ok(Self {
            frame_id: frame.frame_id()?,
            service_id: frame.service_id()?,
            service_type: frame.service_type()?,
            x_id: frame.x_id(),
            response_delay: frame.response_delay(),
            data_length: frame.data_length(),
        })
    }
}

#[cfg(test)]
mod tests {

    use smoltcp::wire::EthernetFrame;

    use crate::dcp::header::{DCPHeaderFrame, FrameID, ServiceID, ServiceType};

    #[test]
    fn test_parse_dcp_header() {
        let raw_packet: [u8; 64] = [
            0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00, 0x52, 0x54, 0x00, 0x8a, 0x3b, 0xa5, 0x88, 0x92,
            0xfe, 0xfe, 0x05, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0xc0, 0x00, 0x04, 0xff, 0xff,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let packet = EthernetFrame::new_checked(raw_packet);

        assert!(packet.is_ok());

        let mut packet = packet.unwrap();

        // println!("Src: {}", packet.src_addr());
        // println!("Dst: {}", packet.dst_addr());
        // println!("Type: {}", packet.ethertype());
        // println!("Payload: {:?}", packet.payload_mut());

        let payload = packet.payload_mut();
        let dcp_header = DCPHeaderFrame::new_checked(payload);

        assert!(dcp_header.is_ok());

        let dcp_header = dcp_header.unwrap();

        assert_eq!(dcp_header.frame_id().unwrap(), FrameID::Request);
        assert_eq!(dcp_header.service_id().unwrap(), ServiceID::Identify);
        assert_eq!(dcp_header.service_type().unwrap(), ServiceType::Request);
        assert_eq!(dcp_header.x_id(), 5);
        assert_eq!(dcp_header.response_delay(), 192);
        assert_eq!(dcp_header.data_length(), 4);

        // println!("Frame ID: {:?}", dcp_header.frame_id().unwrap());
        // println!("Service ID: {:?}", dcp_header.service_id().unwrap());
        // println!("Service Type: {:?}", dcp_header.service_type().unwrap());
        // println!("X ID: {}", dcp_header.x_id());
        // println!("Response Delay: {}", dcp_header.response_delay());
        // println!("Data Length: {}", dcp_header.data_length());
    }
}
