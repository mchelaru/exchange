use cancel::{Cancel, CANCEL_SIZE};
use decoder::Decoder;
use execution_report::{ExecutionReport, EXECUTIONREPORT_SIZE};
use header::{OepHeader, OEP_HEADER_SIZE};
use login::{Login, LOGIN_SIZE};
use modify::{Modify, MODIFY_SIZE};
use neworder::{NewOrder, NEWORDER_SIZE};
use oep_message::{MsgType, OepMessage};

pub mod cancel;
pub mod connection;
pub mod decoder;
pub mod execution_report;
pub mod header;
pub mod login;
pub mod modify;
pub mod neworder;
pub mod oep_message;
pub mod sessioninfo;
pub mod trade;

mod tests;

/// converts Err from std::error::Error to std::io::Error
/// used by oep_decode, since msg::decode can return a broader range of errors
fn convert_decode_error<T>(m: Result<T, Box<dyn std::error::Error>>) -> Result<T, std::io::Error> {
    match m {
        Ok(r) => Ok(r),
        Err(e) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        }
    }
}

fn convert_slicing_error<T, E>(m: Result<T, E>) -> Result<T, std::io::Error>
where
    E: std::error::Error,
{
    match m {
        Ok(r) => Ok(r),
        Err(e) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        }
    }
}

pub fn oep_decode(buffer: &[u8]) -> Result<Box<dyn OepMessage>, std::io::Error> {
    if buffer.len() < OEP_HEADER_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "incomplete",
        ));
    }
    let header_buffer: [u8; OEP_HEADER_SIZE] =
        convert_slicing_error(buffer[..OEP_HEADER_SIZE].try_into())?;
    let header = convert_decode_error(OepHeader::decode(header_buffer))?;
    if buffer.len() < header.msg_len as usize + OEP_HEADER_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "incomplete",
        ));
    }
    match header.message_type() {
        MsgType::Login => {
            let inner_buffer: [u8; LOGIN_SIZE] =
                convert_slicing_error(buffer[OEP_HEADER_SIZE..].try_into())?;
            Ok(Box::new(convert_decode_error(Login::decode(inner_buffer))?))
        }
        MsgType::NewOrder => {
            let inner_buffer: [u8; NEWORDER_SIZE] =
                convert_slicing_error(buffer[OEP_HEADER_SIZE..].try_into())?;
            Ok(Box::new(convert_decode_error(NewOrder::decode(
                inner_buffer,
            ))?))
        }
        MsgType::Modify => {
            let inner_buffer: [u8; MODIFY_SIZE] =
                convert_slicing_error(buffer[OEP_HEADER_SIZE..].try_into())?;
            Ok(Box::new(convert_decode_error(Modify::decode(
                inner_buffer,
            ))?))
        }
        MsgType::Cancel => {
            let inner_buffer: [u8; CANCEL_SIZE] =
                convert_slicing_error(buffer[OEP_HEADER_SIZE..].try_into())?;
            Ok(Box::new(convert_decode_error(Cancel::decode(
                inner_buffer,
            ))?))
        }
        MsgType::ExecutionReport => {
            let inner_buffer: [u8; EXECUTIONREPORT_SIZE] =
                convert_slicing_error(buffer[OEP_HEADER_SIZE..].try_into())?;
            Ok(Box::new(convert_decode_error(ExecutionReport::decode(
                inner_buffer,
            ))?))
        }
        MsgType::Trade => Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Trade cannot be sent on this message pipe",
        )),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Unknown message type",
        )),
    }
}
