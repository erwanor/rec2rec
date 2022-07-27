use anyhow;
use bytes::{Buf, BufMut, BytesMut};
use std::io::{self, Cursor};
use thiserror;
use tokio_util::codec::{Decoder, Encoder};
use tracing::{error, info, warn};

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
pub enum Message {
    Heartbeat(u32),
    Ping,
    Pong,
    Info(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("incomplete message frame")]
    Incomplete,
    #[error("invalid message encoding")]
    InvalidEncoding,
    #[error("bad io")]
    BadIO,
}

impl From<io::Error> for ParseError {
    fn from(_: io::Error) -> Self {
        ParseError::BadIO
    }
}

pub struct MessageCodec {}

impl Encoder<Message> for MessageCodec {
    type Error = io::Error;
    fn encode(&mut self, item: Message, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            Message::Ping => {
                dst.put_u8(b'>');
                dst.extend_from_slice("PING".as_bytes());
            }
            Message::Pong => {
                dst.put_u8(b'>');
                dst.extend_from_slice("PONG".as_bytes());
            }
            Message::Info(s) => {
                if s.len() > MAX_FRAME_LENGTH {
                    // TODO: clean up error propagation / use the correct variant
                    return Err(io::Error::new(io::ErrorKind::Other, "toooooo lonnnngggggg"));
                }

                let len = s.len() as u8;
                dst.put_u8(b'*');
                dst.put_u8(len);
                dst.put_slice(s.as_bytes());
            }
            Message::Heartbeat(clock) => {
                dst.put_u8(b'+');
                dst.extend_from_slice(clock.to_le_bytes().as_slice());
            }
        }

        dst.extend_from_slice("\r\n".as_bytes());
        Ok(())
    }
}

impl Decoder for MessageCodec {
    type Item = Message;
    type Error = ParseError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        let mut cursor = Cursor::new(src[..].as_ref());
        match Message::check(&mut cursor) {
            Ok(_) => {
                let frame_length = cursor.position() as usize;
                cursor.set_position(0);
                match cursor.get_u8() {
                    b'>' => {
                        let content = String::from_utf8(src[1..frame_length - 2].to_vec()).unwrap();
                        match content.as_str() {
                            "PING" => {
                                src.advance(frame_length);
                                return Ok(Some(Message::Ping));
                            }
                            "PONG" => {
                                src.advance(frame_length);
                                return Ok(Some(Message::Pong));
                            }
                            "VARLEN" => {
                                src.advance(frame_length);
                                return Ok(Some(Message::Ping));
                            }
                            _ => {
                                return Err(ParseError::InvalidEncoding);
                            }
                        }
                    }
                    b'*' => {
                        let content = String::from_utf8(src[2..frame_length - 2].to_vec()).unwrap();

                        src.advance(frame_length);
                        return Ok(Some(Message::Info(content)));
                    }
                    b'+' => {
                        let clock: [u8; 4] = (&src[1..frame_length - 2])
                            .try_into()
                            .expect("clock slice to small");
                        let clock = u32::from_le_bytes(clock);
                        src.advance(frame_length);
                        return Ok(Some(Message::Heartbeat(clock)));
                    }
                    _ => {
                        return Err(ParseError::InvalidEncoding);
                    }
                }
            }
            Err(ParseError::Incomplete) => {
                return Ok(None);
            }
            _ => {
                return Err(ParseError::InvalidEncoding);
            }
        }
    }
}

const MAX_FRAME_LENGTH: usize = 64; // 64 bytes

fn get_u8(src: &mut Cursor<&[u8]>) -> Option<u8> {
    if !src.has_remaining() {
        None
    } else {
        Some(src.get_u8())
    }
}

fn get_u32(src: &mut Cursor<&[u8]>) -> Option<u32> {
    if src.remaining() >= 4 {
        Some(src.get_u32())
    } else {
        None
    }
}

impl Message {
    pub fn check(src: &mut Cursor<&[u8]>) -> Result<(), ParseError> {
        if !src.has_remaining() {
            return Err(ParseError::Incomplete);
        }

        match src.get_u8() {
            b'>' => {
                let raw_frame = scan_for_delimiter(src)?;
                if raw_frame.len() > MAX_FRAME_LENGTH {
                    Err(ParseError::InvalidEncoding)
                } else {
                    Ok(())
                }
            }
            b'*' => {
                if let Some(specified_message_length) = get_u8(src) {
                    conservative_scan_for_delimiter(src, specified_message_length as usize)?;
                    return Ok(());
                } else {
                    return Err(ParseError::Incomplete);
                }
            }
            b'+' => {
                if get_u32(src).is_some() {
                    src.set_position(1);
                    conservative_scan_for_delimiter(src, 4)?;
                    return Ok(());
                } else {
                    return Err(ParseError::Incomplete);
                }
            }
            _ => Err(ParseError::InvalidEncoding),
        }
    }
}

struct RawMessage<'a, 'b> {
    delimiter_size: u8,
    max_frame_size: u8,
    cursor: &'b mut Cursor<&'a [u8]>,
}

/// Checks that the specified frame length match the actual buffer length accouting for the
/// protocol prefix, the length specified, and the end of message delimiters. After performing
/// those checks, this method scans the buffer looking for delimiters.
fn conservative_scan_for_delimiter<'a>(
    src: &mut Cursor<&'a [u8]>,
    specified_length: usize,
) -> Result<&'a [u8], ParseError> {
    let buffer_length = src.get_ref().len();
    let index_start = src.position() as usize;
    let index_end = index_start + specified_length;

    let prefix_size = index_start;
    let delimiter_size = 2;

    // Sanity check, is the specified length within boundaries
    if specified_length > MAX_FRAME_LENGTH {
        error!("specified length too big!");
        return Err(ParseError::InvalidEncoding);
    }

    // First, we check that we have enough data in our buffer to even parse the frame according too
    // the client specified_length
    //
    // a raw message:
    //
    //
    //    label         prefix                      frame                       delimiter
    //            ...................  ................................   ....................
    //    index    0           1         2             3           4         5          6
    //   content   *           3         A             B           C         \r         \n
    //             |           |         |             |           |
    //             |           |          --------------------------
    //             |           |                       |- content
    //             |           |
    //             |           |
    //             |           |- frame length
    //             |- message type
    //
    let complete_raw_message_size = prefix_size + specified_length + delimiter_size;
    if buffer_length < complete_raw_message_size {
        warn!("no complete frames found");
        return Err(ParseError::Incomplete);
    }

    // Now that we know that the current buffer has enough data for at least one frame
    // we can check that the delimiters are present where they're supposed to be.
    let inner_buffer = src.get_ref();

    if inner_buffer[index_start + specified_length] != b'\r'
        && inner_buffer[index_start + specified_length + 1] != b'\n'
    {
        error!("invalid protocol message!");
        return Err(ParseError::InvalidEncoding);
    } else {
        // Here, we could perform additional protocol checks
        src.set_position(index_end as u64 + 2);
        return Ok(&src.get_ref()[index_start..index_end]);
    }
}

/// Scans a buffer, looking for a delimiter.
fn scan_for_delimiter<'a>(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], ParseError> {
    let start = src.position() as usize;
    let end = src.get_ref().len() - 1;

    for i in start..end {
        if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
            src.set_position((i + 2) as u64);
            return Ok(&src.get_ref()[start..i]);
        }
    }

    Err(ParseError::Incomplete)
}
