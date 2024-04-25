use byteorder::{ByteOrder, NetworkEndian};
use num_enum::{FromPrimitive, TryFromPrimitive};
use smoltcp::wire::EthernetAddress;

use crate::field::{Field, Rest};

use self::error::EthernetError;

pub mod error;

#[derive(Debug, PartialEq, Clone, FromPrimitive)]
#[repr(u16)]
pub enum EthType {
    Profinet = 0x8892,
    #[num_enum(default)]
    Other,
}

#[derive(Debug, PartialEq, Clone)]
pub enum FrameId {
    Dcp,
    Other,
}

impl From<u16> for FrameId {
    fn from(value: u16) -> Self {
        match value {
            0xfefc..=0xfeff => Self::Dcp,
            _ => Self::Other,
        }
    }
}

pub struct EthernetFrame<T: AsRef<[u8]>> {
    buffer: T,
}

impl<T: AsRef<[u8]>> EthernetFrame<T> {
    const DESTINATION_FIELD: Field = 0..6;
    const SOURCE_FIELD: Field = 6..12;
    const TYPE_FIELD: Field = 12..14;
    const FRAME_ID_FIELD: Field = 14..16;
    const PAYLOAD_FIELD: Rest = 16..;

    pub fn new_unchecked(buffer: T) -> Self {
        Self { buffer }
    }

    pub fn new_checked(buffer: T) -> Result<Self, EthernetError> {
        let packet = Self::new_unchecked(buffer);
        packet.check_len()?;
        Ok(packet)
    }

    pub fn check_len(&self) -> Result<(), EthernetError> {
        let len = self.buffer.as_ref().len();

        if len < Self::PAYLOAD_FIELD.start {
            Err(EthernetError::PacketParsingError)
        } else {
            Ok(())
        }
    }

    pub fn dst_address(&self) -> EthernetAddress {
        let data = self.buffer.as_ref();
        EthernetAddress::from_bytes(&data[Self::DESTINATION_FIELD])
    }

    pub fn src_address(&self) -> EthernetAddress {
        let data = self.buffer.as_ref();
        EthernetAddress::from_bytes(&data[Self::SOURCE_FIELD])
    }

    pub fn eth_type(&self) -> EthType {
        let data = self.buffer.as_ref();
        let raw = NetworkEndian::read_u16(&data[Self::TYPE_FIELD]);
        EthType::from(raw)
    }

    pub fn is_profinet(&self) -> bool {
        self.eth_type() == EthType::Profinet
    }

    pub fn frame_id(&self) -> FrameId {
        let data = self.buffer.as_ref();
        let raw = NetworkEndian::read_u16(&data[Self::FRAME_ID_FIELD]);
        FrameId::from(raw)
    }

    pub fn payload(&self) -> &[u8] {
        let data = self.buffer.as_ref();
        &data[Self::PAYLOAD_FIELD]
    }
}

pub fn handle_incoming_packet(packet: &[u8]) -> Result<(), EthernetError> {
    let frame =
        EthernetFrame::new_checked(packet).map_err(|_| EthernetError::PacketParsingError)?;

    todo!()
}
