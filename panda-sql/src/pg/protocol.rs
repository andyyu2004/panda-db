use std::io;

use anyhow::ensure;
use byteorder::{BigEndian, ByteOrder, ReadBytesExt};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use panda_db::{PandaError, PandaResult};
use postgres_protocol::message::backend::{self, *};
use tokio_util::codec::{Decoder, Encoder};

const SSL_MAGIC: [u8; 4] = u32::to_be_bytes(80877103);
const PARSE_TAG: u8 = b'P';
const QUERY_TAG: u8 = b'Q';
const TERMINATE_TAG: u8 = b'X';

#[derive(Default)]
pub(super) struct PgCodec {
    /// false initially for `StartupMessage` and `SSLRequest`
    /// as they lack the leading type byte
    started: bool,
}

pub(super) enum BackendMessage {
    AuthenticationOk,
    Message(backend::Message),
    CommandComplete,
    EmptyQueryResponse,
    ParameterStatus(String, String),
    ReadyForQuery(ReadyForQueryStatus),
    Raw(Bytes),
}

#[derive(Debug, PartialEq)]
#[repr(u8)]
pub(super) enum ReadyForQueryStatus {
    Idle  = b'I',
    Tran  = b'T',
    Error = b'E',
}

#[derive(Debug)]
pub(super) enum FrontendMessage {
    Query(String),
    SSLRequest,
    StartupMessage { protocol_version: u32, parameters: Vec<(String, String)> },
    Terminate,
}

impl Encoder<BackendMessage> for PgCodec {
    type Error = PandaError;

    fn encode(&mut self, msg: BackendMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut write = |tag: u8, data: &[u8]| {
            dst.reserve(5 + data.len());
            dst.put_u8(tag);
            dst.put_u32(4 + data.len() as u32);
            dst.extend_from_slice(data);
        };

        macro_rules! write_cstr {
            ($s:expr) => {{
                let s = $s.as_bytes();
                if s.contains(&0) {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "string contains embedded null",
                    ))?;
                }
                dst.extend_from_slice(s);
                dst.put_u8(0);
            }};
        }

        macro_rules! write_msg {
            ($tag:expr, $msg:expr) => {
                write($tag, $msg)
            };
            ($tag:expr) => {
                write($tag, &[])
            };
        }

        match msg {
            BackendMessage::Raw(bytes) => dst.extend_from_slice(&bytes),
            BackendMessage::AuthenticationOk =>
                write_msg!(AUTHENTICATION_TAG, &u32::to_be_bytes(0)),
            BackendMessage::ReadyForQuery(status) =>
                write_msg!(READY_FOR_QUERY_TAG, &[status as u8]),
            BackendMessage::EmptyQueryResponse => write_msg!(EMPTY_QUERY_RESPONSE_TAG),
            BackendMessage::Message(_) => todo!(),
            BackendMessage::CommandComplete => todo!(),
            BackendMessage::ParameterStatus(key, value) => {
                write_msg!(PARAMETER_STATUS_TAG);
                write_cstr!(&key);
                write_cstr!(&value);
            }
        }
        Ok(())
    }
}

impl Decoder for PgCodec {
    type Error = PandaError;
    type Item = FrontendMessage;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if !self.started {
            if src.is_empty() {
                return Ok(None);
            }
            let len = BigEndian::read_u32(src) as usize;
            let src = src.split_to(len);
            let mut bytes = &src[4..];
            let msg = if bytes == SSL_MAGIC {
                FrontendMessage::SSLRequest
            } else {
                self.started = true;
                let protocol_version = bytes.read_u32::<BigEndian>()?;
                ensure!(
                    protocol_version == 0x30000,
                    "unsupported protocol version `{}`; protocol v3.0 supported",
                    protocol_version
                );

                let mut parameters = vec![];
                while bytes[0] != 0 {
                    let key = read_cstr(&mut bytes)?.to_owned();
                    let value = read_cstr(&mut bytes)?.to_owned();
                    parameters.push((key, value));
                }
                assert_eq!(bytes.len(), 1);
                FrontendMessage::StartupMessage { protocol_version, parameters }
            };
            return Ok(Some(msg));
        }

        let header = match Header::parse(src)? {
            Some(header) => header,
            None => return Ok(None),
        };
        src.advance(1 + header.len() as usize);

        let data = &src[..header.len() as usize - 4];
        let msg = match header.tag() {
            QUERY_TAG => FrontendMessage::Query(std::str::from_utf8(data)?.to_owned()),
            PARSE_TAG => todo!("parse"),
            TERMINATE_TAG => FrontendMessage::Terminate,
            tag => panic!("unsupported tag {}", tag),
        };
        Ok(Some(msg))
    }
}

fn read_cstr<'a>(bytes: &mut &'a [u8]) -> PandaResult<&'a str> {
    match memchr::memchr(0, bytes) {
        Some(idx) => {
            let s = &bytes[..idx];
            *bytes = &bytes[idx + 1..];
            Ok(std::str::from_utf8(s)?)
        }
        None => unreachable!(),
    }
}
