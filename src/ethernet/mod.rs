use byteorder::{ByteOrder, NetworkEndian};
use defmt::Format;
use num_enum::FromPrimitive;
use smoltcp::wire::EthernetAddress;

use crate::field::{Field, Rest};

use self::eth_dma::{RxError, TxError};

pub mod eth_dma;

#[derive(Debug, PartialEq, Clone, FromPrimitive)]
#[repr(u16)]
pub enum EthType {
    Profinet = 0x8892,
    Vlan = 0x8100,
    #[num_enum(default)]
    Other,
}

#[derive(Debug, PartialEq, Clone, FromPrimitive)]
#[repr(u16)]
pub enum FrameId {
    #[num_enum(default)]
    Other,
    #[num_enum(alternatives = [0xfefd..0xfeff])]
    Dcp = 0xfefc,
}

#[derive(Debug, Format)]
pub enum EthernetError {
    PacketParsingError,
    RxError(RxError),
    TxError(TxError),
}

#[derive(Debug)]
pub struct EthernetFrame<T: AsRef<[u8]>> {
    buffer: T,
    is_vlan: bool,
}

impl<T: AsRef<[u8]>> EthernetFrame<T> {
    const DESTINATION_FIELD: Field = 0..6;
    const SOURCE_FIELD: Field = 6..12;
    const TYPE_FIELD: Field = 12..14;
    const FRAME_ID_FIELD: Field = 14..16;
    const PAYLOAD_FIELD: Rest = 16..;
    const VLAN_TYPE_FIELD: Field = 16..18;
    const VLAN_FRAME_ID: Field = 18..20;
    const VLAN_PAYLOAD_FIELD: Rest = 20..;

    pub fn new_unchecked(buffer: T) -> Self {
        Self {
            buffer,
            is_vlan: false,
        }
    }

    pub fn new_checked(buffer: T) -> Result<Self, EthernetError> {
        let mut packet = Self::new_unchecked(buffer);
        packet.check_len()?;

        packet.is_vlan = packet.is_vlan();
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

        let raw = if self.is_vlan {
            NetworkEndian::read_u16(&data[Self::VLAN_TYPE_FIELD])
        } else {
            NetworkEndian::read_u16(&data[Self::TYPE_FIELD])
        };

        EthType::from(raw)
    }

    pub fn is_profinet(&self) -> bool {
        self.eth_type() == EthType::Profinet
    }

    pub fn is_vlan(&self) -> bool {
        self.eth_type() == EthType::Vlan
    }

    pub fn frame_id(&self) -> FrameId {
        FrameId::from(self.frame_id_u16())
    }

    pub fn frame_id_u16(&self) -> u16 {
        let data = self.buffer.as_ref();

        if self.is_vlan {
            NetworkEndian::read_u16(&data[Self::VLAN_FRAME_ID])
        } else {
            NetworkEndian::read_u16(&data[Self::FRAME_ID_FIELD])
        }
    }

    pub fn payload(&self) -> &[u8] {
        let data = self.buffer.as_ref();

        if self.is_vlan {
            &data[Self::VLAN_PAYLOAD_FIELD]
        } else {
            &data[Self::PAYLOAD_FIELD]
        }
    }
}
