use defmt::Format;

pub struct PacketId(pub u32);

#[derive(Debug, Format)]
pub enum RxError {
    /// The received packet was truncated
    Truncated,
    /// An error occured with the DMA
    DmaError,
    /// Receiving would block
    WouldBlock,
}

#[derive(Debug, Format)]
pub enum TxError {
    /// Ring buffer is full
    WouldBlock,
}

pub trait EthernetDMA {
    fn recv_next(&mut self, packet_id: Option<PacketId>) -> Result<[u8; 1024], RxError>;
    fn send<F>(&mut self, length: usize, packet_id: Option<PacketId>, f: F) -> Result<(), TxError>
    where
        F: FnOnce(&mut [u8]);
}

impl<'rx, 'tx> EthernetDMA for stm32_eth::dma::EthernetDMA<'rx, 'tx> {
    fn recv_next(&mut self, packet_id: Option<PacketId>) -> Result<[u8; 1024], RxError> {
        let mut buf = [0; 1024];

        let res = self
            .recv_next(packet_id.map(|id| stm32_eth::dma::PacketId(id.0)))
            .map_err(|e| match e {
                stm32_eth::dma::RxError::Truncated => RxError::Truncated,
                stm32_eth::dma::RxError::DmaError => RxError::DmaError,
                stm32_eth::dma::RxError::WouldBlock => RxError::WouldBlock,
            })?;

        buf.copy_from_slice(&*res);

        Ok(buf)
    }

    fn send<F>(&mut self, length: usize, packet_id: Option<PacketId>, f: F) -> Result<(), TxError>
    where
        F: FnOnce(&mut [u8]),
    {
        self.send(
            length,
            packet_id.map(|id| stm32_eth::dma::PacketId(id.0)),
            f,
        )
        .map_err(|e| match e {
            stm32_eth::dma::TxError::WouldBlock => TxError::WouldBlock,
        })
    }
}
