pub struct Address(pub [u8; 6]);

pub enum EtherType {
    IPV4,
    ARP,
    Profinet,
    LLDP,
}

pub struct EthernetHeader {
    pub src: Address,
    pub dst: Address,
    pub ether_type: EtherType,
}

pub enum FrameData {
    ProfinetDcp,
}

pub struct EthernetTrailer {
    pub frame_check_seq: [u8; 4],
}

pub struct EthernetFrame {
    pub header: EthernetHeader,
    pub data: FrameData,
    pub trailer: EthernetTrailer,
}
