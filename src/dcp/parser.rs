use byteorder::{ByteOrder, NetworkEndian};
use nom::Err::Error;
use nom::{bytes::complete::take, IResult};
use num_enum::TryFromPrimitive;

use crate::ServiceType;
use crate::{dcp::error::ParseDcpHeaderError, ParseDcpError, ServiceId};

fn takeu8(i: &[u8]) -> IResult<&[u8], u8, ParseDcpError<&[u8]>> {
    let (i, buf) = take(1usize)(i)?;
    Ok((i, buf[0]))
}

fn takeu16(i: &[u8]) -> IResult<&[u8], u16, ParseDcpError<&[u8]>> {
    let (i, buf) = take(2usize)(i)?;
    let num = NetworkEndian::read_u16(buf);
    Ok((i, num))
}

fn takeu32(i: &[u8]) -> IResult<&[u8], u32, ParseDcpError<&[u8]>> {
    let (i, buf) = take(4usize)(i)?;
    let num = NetworkEndian::read_u32(buf);
    Ok((i, num))
}

fn service_id(i: &[u8]) -> IResult<&[u8], ServiceId, ParseDcpError<&[u8]>> {
    let (i, num) = takeu8(i)?;
    if let Ok(service_id) = ServiceId::try_from_primitive(num) {
        Ok((i, service_id))
    } else {
        Err(Error(ParseDcpError::HeaderError(
            ParseDcpHeaderError::InvalidServiceID,
        )))
    }
}

fn service_type(i: &[u8]) -> IResult<&[u8], ServiceType, ParseDcpError<&[u8]>> {
    let (i, num) = takeu8(i)?;

    if let Ok(service_type) = ServiceType::try_from_primitive(num) {
        Ok((i, service_type))
    } else {
        Err(Error(ParseDcpError::HeaderError(
            ParseDcpHeaderError::InvalidServiceType,
        )))
    }
}

fn x_id(i: &[u8]) -> IResult<&[u8], u32, ParseDcpError<&[u8]>> {
    takeu32(i)
}

fn response_delay(i: &[u8]) -> IResult<&[u8], u16, ParseDcpError<&[u8]>> {
    takeu16(i)
}

fn data_length(i: &[u8]) -> IResult<&[u8], u16, ParseDcpError<&[u8]>> {
    takeu16(i)
}
