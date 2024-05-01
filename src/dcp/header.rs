use byteorder::{ByteOrder, NetworkEndian};
use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::field::{Field, Rest, SmallField};

use super::error::ParseDcpHeaderError;

const SERVICE_ID_FIELD: SmallField = 0;
const SERVICE_TYPE_FIELD: SmallField = 1;
const X_ID_FIELD: Field = 2..6;
const RESPONSE_DELAY_FIELD: Field = 6..8;
const DATA_LENGTH_FIELD: Field = 8..10;
const PAYLOAD_FIELD: Rest = 10..;
pub const DCP_HEADER_LENGTH_FIELD: usize = PAYLOAD_FIELD.start;

#[derive(Debug, PartialEq, Clone, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum ServiceType {
    Request = 0,
    Success = 1,
    NotSupported = 5,
}

#[derive(Debug, PartialEq, Clone, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum ServiceId {
    Get = 3,
    Set = 4,
    Identify = 5,
    Hello = 6,
}

pub struct DcpHeaderFrame<T: AsRef<[u8]>> {
    buffer: T,
}

impl<T: AsRef<[u8]>> DcpHeaderFrame<T> {
    pub const fn new_unchecked(buffer: T) -> Self {
        DcpHeaderFrame { buffer }
    }

    pub fn new_checked(buffer: T) -> Result<Self, ParseDcpHeaderError> {
        let header = Self::new_unchecked(buffer);
        match header.check_len() {
            true => Ok(header),
            false => Err(ParseDcpHeaderError::InvalidHeaderLength),
        }
    }

    pub fn check_len(&self) -> bool {
        let len = self.buffer.as_ref().len();

        len > DCP_HEADER_LENGTH_FIELD
    }

    pub fn service_id(&self) -> Result<ServiceId, ParseDcpHeaderError> {
        let data = self.buffer.as_ref();
        let raw = data[SERVICE_ID_FIELD];
        ServiceId::try_from_primitive(raw).map_err(|_| ParseDcpHeaderError::InvalidServiceID)
    }

    pub fn service_type(&self) -> Result<ServiceType, ParseDcpHeaderError> {
        let data = self.buffer.as_ref();
        let raw = data[SERVICE_TYPE_FIELD];
        ServiceType::try_from_primitive(raw).map_err(|_| ParseDcpHeaderError::InvalidServiceType)
    }

    pub fn x_id(&self) -> u32 {
        let data = self.buffer.as_ref();
        NetworkEndian::read_u32(&data[X_ID_FIELD])
    }

    pub fn response_delay(&self) -> u16 {
        let data = self.buffer.as_ref();
        NetworkEndian::read_u16(&data[RESPONSE_DELAY_FIELD])
    }

    pub fn data_length(&self) -> u16 {
        let data = self.buffer.as_ref();
        NetworkEndian::read_u16(&data[DATA_LENGTH_FIELD])
    }

    pub fn payload(&self) -> &[u8] {
        let data = self.buffer.as_ref();
        &data[PAYLOAD_FIELD]
    }
}

pub struct DcpHeader {
    pub service_id: ServiceId,
    pub service_type: ServiceType,
    pub x_id: u32,
    pub response_delay_factor: u16,
    pub data_length: u16,
}

impl DcpHeader {
    pub fn new(
        service_id: ServiceId,
        service_type: ServiceType,
        x_id: u32,
        response_delay: u16,
    ) -> Self {
        Self {
            service_id,
            service_type,
            x_id,
            response_delay_factor: response_delay,
            data_length: 0,
        }
    }

    pub fn parse<T: AsRef<[u8]>>(frame: &DcpHeaderFrame<T>) -> Result<Self, ParseDcpHeaderError> {
        Ok(Self {
            service_id: frame.service_id()?,
            service_type: frame.service_type()?,
            x_id: frame.x_id(),
            response_delay_factor: frame.response_delay(),
            data_length: frame.data_length(),
        })
    }

    pub fn encode_into(&self, buffer: &mut [u8]) {
        buffer[SERVICE_ID_FIELD] = self.service_id.clone().into();
        buffer[SERVICE_TYPE_FIELD] = self.service_type.clone().into();
        NetworkEndian::write_u32(&mut buffer[X_ID_FIELD], self.x_id);
        NetworkEndian::write_u16(&mut buffer[RESPONSE_DELAY_FIELD], 0);
        NetworkEndian::write_u16(&mut buffer[DATA_LENGTH_FIELD], self.data_length);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        dcp::header::{DcpHeaderFrame, ServiceId, ServiceType},
        ethernet::EthernetFrame,
        DcpHeader,
    };

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

        let packet = packet.unwrap();

        // println!("Src: {}", packet.src_addr());
        // println!("Dst: {}", packet.dst_addr());
        // println!("Type: {}", packet.ethertype());
        // println!("Payload: {:?}", packet.payload_mut());

        let payload = packet.payload();
        let dcp_header = DcpHeaderFrame::new_checked(payload);

        assert!(dcp_header.is_ok());

        let dcp_header = dcp_header.unwrap();

        assert_eq!(dcp_header.service_id().unwrap(), ServiceId::Identify);
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

    #[test]
    fn test_encode_header() {
        let header = DcpHeader::new(ServiceId::Identify, ServiceType::Success, 1, 0);
        assert_eq!(header.data_length, 0);

        let mut buffer = [0; 10];
        header.encode_into(&mut buffer);

        assert_eq!(buffer, [5, 1, 0, 0, 0, 1, 0, 0, 0, 0]);
    }
}
