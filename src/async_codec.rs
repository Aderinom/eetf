use super::*;
use crate::codec::Decoder;
use crate::codec::Encoder;
use crate::codec_common::*;
use crate::convert::TryAsRef;
use byteorder::BigEndian;
use byteorder::WriteBytesExt;
use num::bigint::BigInt;
use std::convert::From;
use std::io;
use std::io::Write;
use std::str;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use async_recursion::async_recursion;

pub struct AsyncDecoder<R> {
    reader: R,
    buf: Vec<u8>,
}
impl<R: tokio::io::AsyncRead + std::marker::Unpin  + std::marker::Send>   AsyncDecoder<R> {
    pub fn new(reader: R) -> Self {
        AsyncDecoder {
            reader,
            buf: Vec::new(),
        }
    }
    pub async fn decode(mut self) -> DecodeResult {
        let version = self.reader.read_u8().await?;
        if version != VERSION {
            return Err(DecodeError::UnsupportedVersion { version });
        }
        let tag = self.reader.read_u8().await?;
        match tag {
            COMPRESSED_TERM => self.decode_compressed_term().await,
            DISTRIBUTION_HEADER => unimplemented!(),
            _ => self.decode_term_with_tag(tag).await,
        }
    }
    async fn decode_term(&mut self) -> DecodeResult {
        let tag = self.reader.read_u8().await?;
        self.decode_term_with_tag(tag).await
    }
    #[async_recursion]
    async fn decode_term_with_tag(&mut self, tag: u8) -> DecodeResult {
        match tag {
            NEW_FLOAT_EXT => self.decode_new_float_ext().await,
            BIT_BINARY_EXT => self.decode_bit_binary_ext().await,
            ATOM_CACHE_REF => unimplemented!(),
            SMALL_INTEGER_EXT => self.decode_small_integer_ext().await,
            INTEGER_EXT => self.decode_integer_ext().await,
            FLOAT_EXT => self.decode_float_ext().await,
            ATOM_EXT => self.decode_atom_ext().await,
            REFERENCE_EXT => self.decode_reference_ext().await,
            PORT_EXT => self.decode_port_ext().await,
            NEW_PORT_EXT => self.decode_new_port_ext().await,
            V4_PORT_EXT => self.decode_v4_port_ext().await,
            PID_EXT => self.decode_pid_ext().await,
            NEW_PID_EXT => self.decode_new_pid_ext().await,
            SMALL_TUPLE_EXT => self.decode_small_tuple_ext().await,
            LARGE_TUPLE_EXT => self.decode_large_tuple_ext().await,
            NIL_EXT => self.decode_nil_ext().await,
            STRING_EXT => self.decode_string_ext().await,
            LIST_EXT => self.decode_list_ext().await,
            BINARY_EXT => self.decode_binary_ext().await,
            SMALL_BIG_EXT => self.decode_small_big_ext().await,
            LARGE_BIG_EXT => self.decode_large_big_ext().await,
            NEW_FUN_EXT => self.decode_new_fun_ext().await,
            EXPORT_EXT => self.decode_export_ext().await,
            NEW_REFERENCE_EXT => self.decode_new_reference_ext().await,
            SMALL_ATOM_EXT => self.decode_small_atom_ext().await,
            MAP_EXT => self.decode_map_ext().await,
            FUN_EXT => self.decode_fun_ext().await,
            ATOM_UTF8_EXT => self.decode_atom_utf8_ext().await,
            SMALL_ATOM_UTF8_EXT => self.decode_small_atom_utf8_ext().await,
            NEWER_REFERENCE_EXT => self.decode_newer_reference_ext().await,
            _ => Err(DecodeError::UnknownTag { tag }),
        }
    }
    async fn decode_compressed_term(&mut self) -> DecodeResult {
        unimplemented!()
    }
    #[allow(clippy::unnecessary_wraps)]
    async fn decode_nil_ext(&mut self) -> DecodeResult {
        Ok(Term::from(List::nil()))
    }
    async fn decode_string_ext(&mut self) -> DecodeResult {
            let size = self.reader.read_u16().await? as usize;
            let mut bytes = vec![0; size];
            self.reader.read_exact(&mut bytes).await?;
            Ok(Term::from(ByteList::from(bytes)))            
    }
    async fn decode_list_ext(&mut self) -> DecodeResult {
        let count = self.reader.read_u32().await? as usize;
        let mut elements = Vec::with_capacity(count);
        for _ in 0..count {
            elements.push(self.decode_term().await?);
        }
        let last = self.decode_term().await?;
        if last.try_as_ref().map(List::is_nil).unwrap_or(false) {
            Ok(Term::from(List::from(elements)))
        } else {
            Ok(Term::from(ImproperList::from((elements, last))))
        }
    }
    async fn decode_small_tuple_ext(&mut self) -> DecodeResult {
        let count = self.reader.read_u8().await? as usize;
        let mut elements = Vec::with_capacity(count);
        for _ in 0..count {
            elements.push(self.decode_term().await?);
        }
        Ok(Term::from(Tuple::from(elements)))
    }
    async fn decode_large_tuple_ext(&mut self) -> DecodeResult {
        let count = self.reader.read_u32().await? as usize;
        let mut elements = Vec::with_capacity(count);
        for _ in 0..count {
            elements.push(self.decode_term().await?);
        }
        Ok(Term::from(Tuple::from(elements)))
    }
    async fn decode_map_ext(&mut self) -> DecodeResult {
        let count = self.reader.read_u32().await? as usize;
        let mut map = HashMap::<Term,Term>::new();
        for _ in 0..count {
            let k = self.decode_term().await?;
            let v = self.decode_term().await?;
            map.insert(k, v);
        }
        Ok(Term::from(Map::from(map)))
    }
    async fn decode_binary_ext(&mut self) -> DecodeResult {
        let size = self.reader.read_u32().await? as usize;
        let mut buf = vec![0; size];
        self.reader.read_exact(&mut buf).await?;
        Ok(Term::from(Binary::from(buf)))
    }
    async fn decode_bit_binary_ext(&mut self) -> DecodeResult {
        let size = self.reader.read_u32().await? as usize;
        let tail_bits_size = self.reader.read_u8().await?;
        let mut buf = vec![0; size];
        self.reader.read_exact(&mut buf).await?;
        if !buf.is_empty() {
            let last = buf[size - 1] >> (8 - tail_bits_size);
            buf[size - 1] = last;
        }
        Ok(Term::from(BitBinary::from((buf, tail_bits_size))))
    }
    async fn decode_pid_ext(&mut self) -> DecodeResult {
        let node = self.decode_term().await.and_then(aux::term_into_atom)?;
        Ok(Term::from(Pid {
            node,
            id: self.reader.read_u32().await?,
            serial: self.reader.read_u32().await?,
            creation: self.reader.read_u8().await? as u32,
        }))
    }
    async fn decode_new_pid_ext(&mut self) -> DecodeResult {
        let node = self.decode_term().await.and_then(aux::term_into_atom)?;
        Ok(Term::from(Pid {
            node,
            id: self.reader.read_u32().await?,
            serial: self.reader.read_u32().await?,
            creation: self.reader.read_u32().await?,
        }))
    }
    async fn decode_port_ext(&mut self) -> DecodeResult {
        let node: Atom = self.decode_term().await.and_then(|t| {
            t.try_into().map_err(|t| DecodeError::UnexpectedType {
                value: t,
                expected: "Atom".to_string(),
            })
        })?;
        Ok(Term::from(Port {
            node,
            id: u64::from(self.reader.read_u32().await?),
            creation: u32::from(self.reader.read_u8().await?),
        }))
    }
    async fn decode_new_port_ext(&mut self) -> DecodeResult {
        let node: Atom = self.decode_term().await.and_then(|t| {
            t.try_into().map_err(|t| DecodeError::UnexpectedType {
                value: t,
                expected: "Atom".to_string(),
            })
        })?;
        Ok(Term::from(Port {
            node,
            id: u64::from(self.reader.read_u32().await?),
            creation: self.reader.read_u32().await?,
        }))
    }
    async fn decode_v4_port_ext(&mut self) -> DecodeResult {
        let node: Atom = self.decode_term().await.and_then(|t| {
            t.try_into().map_err(|t| DecodeError::UnexpectedType {
                value: t,
                expected: "Atom".to_string(),
            })
        })?;
        Ok(Term::from(Port {
            node,
            id: self.reader.read_u64().await?,
            creation: self.reader.read_u32().await?,
        }))
    }
    async fn decode_reference_ext(&mut self) -> DecodeResult {
        let node = self.decode_term().await.and_then(aux::term_into_atom)?;
        Ok(Term::from(Reference {
            node,
            id: vec![self.reader.read_u32().await?],
            creation: u32::from(self.reader.read_u8().await?),
        }))
    }
    async fn decode_new_reference_ext(&mut self) -> DecodeResult {
        let id_count = self.reader.read_u16().await? as usize;
        let node = self.decode_term().await.and_then(aux::term_into_atom)?;
        let creation = u32::from(self.reader.read_u8().await?);
        let mut id = Vec::with_capacity(id_count);
        for _ in 0..id_count {
            id.push(self.reader.read_u32().await?);
        }
        Ok(Term::from(Reference { node, id, creation }))
    }
    async fn decode_newer_reference_ext(&mut self) -> DecodeResult {
        let id_count = self.reader.read_u16().await? as usize;
        let node = self.decode_term().await.and_then(aux::term_into_atom)?;
        let creation = self.reader.read_u32().await?;
        let mut id = Vec::with_capacity(id_count);
        for _ in 0..id_count {
            id.push(self.reader.read_u32().await?);
        }
        Ok(Term::from(Reference { node, id, creation }))
    }
    async fn decode_export_ext(&mut self) -> DecodeResult {
        let module = self.decode_term().await.and_then(aux::term_into_atom)?;
        let function = self.decode_term().await.and_then(aux::term_into_atom)?;
        let arity = self
            .decode_term().await
            .and_then(|t| aux::term_into_ranged_integer(t, 0..0xFF))? as u8;
        Ok(Term::from(ExternalFun {
            module,
            function,
            arity,
        }))
    }
    async fn decode_fun_ext(&mut self) -> DecodeResult {
        let num_free = self.reader.read_u32().await?;
        let pid = self.decode_term().await.and_then(aux::term_into_pid)?;
        let module = self.decode_term().await.and_then(aux::term_into_atom)?;
        let index = self.decode_term().await.and_then(aux::term_into_fix_integer)?;
        let uniq = self.decode_term().await.and_then(aux::term_into_fix_integer)?;
        let mut vars = Vec::with_capacity(num_free as usize);
        for _ in 0..num_free {
            vars.push(self.decode_term().await?);
        }
        Ok(Term::from(InternalFun::Old {
            module,
            pid,
            free_vars: vars,
            index: index.value,
            uniq: uniq.value,
        }))
    }
    async fn decode_new_fun_ext(&mut self) -> DecodeResult {
        let _size = self.reader.read_u32().await?;
        let arity = self.reader.read_u8().await?;
        let mut uniq = [0; 16];
        self.reader.read_exact(&mut uniq).await?;
        let index = self.reader.read_u32().await?;
        let num_free = self.reader.read_u32().await?;
        let module = self.decode_term().await.and_then(aux::term_into_atom)?;
        let old_index = self.decode_term().await.and_then(aux::term_into_fix_integer)?;
        let old_uniq = self.decode_term().await.and_then(aux::term_into_fix_integer)?;
        let pid = self.decode_term().await.and_then(aux::term_into_pid)?;
        let mut vars = Vec::with_capacity(num_free as usize);
        for _ in 0..num_free {
            vars.push(self.decode_term().await?);
        }
        Ok(Term::from(InternalFun::New {
            module,
            arity,
            pid,
            free_vars: vars,
            index,
            uniq,
            old_index: old_index.value,
            old_uniq: old_uniq.value,
        }))
    }
    async fn decode_new_float_ext(&mut self) -> DecodeResult {
        let value = self.reader.read_f64().await?;
        Ok(Term::from(Float::try_from(value)?))
    }
    async fn decode_float_ext(&mut self) -> DecodeResult {
        let mut buf = [0; 31];
        self.reader.read_exact(&mut buf).await?;
        let float_str = str::from_utf8(&buf)
            .or_else(|e| aux::invalid_data_error(e.to_string()))?
            .trim_end_matches(0 as char);
        let value = float_str
            .parse::<f32>()
            .or_else(|e| aux::invalid_data_error(e.to_string()))?;
        Ok(Term::from(Float::try_from(value)?))
    }
    async fn decode_small_integer_ext(&mut self) -> DecodeResult {
        let value = self.reader.read_u8().await?;
        Ok(Term::from(FixInteger::from(i32::from(value))))
    }
    async fn decode_integer_ext(&mut self) -> DecodeResult {
        let value = self.reader.read_i32().await?;
        Ok(Term::from(FixInteger::from(value)))
    }
    async fn decode_small_big_ext(&mut self) -> DecodeResult {
        let count = self.reader.read_u8().await? as usize;
        let sign = self.reader.read_u8().await?;
        self.buf.resize(count, 0);
        self.reader.read_exact(&mut self.buf).await?;
        let value = BigInt::from_bytes_le(aux::byte_to_sign(sign)?, &self.buf);
        Ok(Term::from(BigInteger { value }))
    }
    async fn decode_large_big_ext(&mut self) -> DecodeResult {
        let count = self.reader.read_u32().await? as usize;
        let sign = self.reader.read_u8().await?;
        self.buf.resize(count, 0);
        self.reader.read_exact(&mut self.buf).await?;
        let value = BigInt::from_bytes_le(aux::byte_to_sign(sign)?, &self.buf);
        Ok(Term::from(BigInteger { value }))
    }
    async fn decode_atom_ext(&mut self) -> DecodeResult {
        let len = self.reader.read_u16().await?;
        self.buf.resize(len as usize, 0);
        self.reader.read_exact(&mut self.buf).await?;
        let name = aux::latin1_bytes_to_string(&self.buf)?;
        Ok(Term::from(Atom { name }))
    }
    async fn decode_small_atom_ext(&mut self) -> DecodeResult {
        let len = self.reader.read_u8().await?;
        self.buf.resize(len as usize, 0);
        self.reader.read_exact(&mut self.buf).await?;
        let name = aux::latin1_bytes_to_string(&self.buf)?;
        Ok(Term::from(Atom { name }))
    }
    async fn decode_atom_utf8_ext(&mut self) -> DecodeResult {
        let len = self.reader.read_u16().await?;
        self.buf.resize(len as usize, 0);
        self.reader.read_exact(&mut self.buf).await?;
        let name = str::from_utf8(&self.buf).or_else(|e| aux::invalid_data_error(e.to_string()))?;
        Ok(Term::from(Atom::from(name)))
    }
    async fn decode_small_atom_utf8_ext(&mut self) -> DecodeResult {
        let len = self.reader.read_u8().await?;
        self.buf.resize(len as usize, 0);
        self.reader.read_exact(&mut self.buf).await?;
        let name = str::from_utf8(&self.buf).or_else(|e| aux::invalid_data_error(e.to_string()))?;
        Ok(Term::from(Atom::from(name)))
    }
}

pub struct AsyncEncoder<W> {
    writer: W,
}
impl<W: tokio::io::AsyncWrite + std::marker::Unpin + Send> AsyncEncoder<W> {
    pub fn new(writer: W) -> Self {
        AsyncEncoder { writer: writer}
    }
    pub async fn encode(mut self, term: &Term) -> EncodeResult {
        self.writer.write_u8(VERSION).await?;
        self.encode_term(term).await
    }
    
    #[async_recursion]
    async fn encode_term(&mut self, term: &Term) -> EncodeResult {
        match *term {
            Term::Atom(ref x) => self.encode_atom(x).await,
            Term::FixInteger(ref x) => self.encode_fix_integer(x).await,
            Term::BigInteger(ref x) => self.encode_big_integer(x).await,
            Term::Float(ref x) => self.encode_float(x).await,
            Term::Pid(ref x) => self.encode_pid(x).await,
            Term::Port(ref x) => self.encode_port(x).await,
            Term::Reference(ref x) => self.encode_reference(x).await,
            Term::ExternalFun(ref x) => self.encode_external_fun(x).await,
            Term::InternalFun(ref x) => self.encode_internal_fun(x).await,
            Term::Binary(ref x) => self.encode_binary(x).await,
            Term::BitBinary(ref x) => self.encode_bit_binary(x).await,
            Term::List(ref x) => self.encode_list(x).await,
            Term::ImproperList(ref x) => self.encode_improper_list(x).await,
            Term::Tuple(ref x) => self.encode_tuple(x).await,
            Term::Map(ref x) => self.encode_map(x).await,
            Term::ByteList(ref x) => self.encode_byte_list(x.bytes.as_slice()).await
        }
    }
    async fn encode_nil(&mut self) -> EncodeResult {
        self.writer.write_u8(NIL_EXT).await?;
        Ok(())
    }
    async fn encode_list(&mut self, x: &List) -> EncodeResult {
        let to_byte = |e: &Term| {
            e.try_as_ref()
                .and_then(|&FixInteger { value: i }| if i < 0x100 { Some(i as u8) } else { None })
        };
        if !x.elements.is_empty()
            && x.elements.len() <= std::u16::MAX as usize
            && x.elements.iter().all(|e| to_byte(e).is_some())
        {
            self.writer.write_u8(STRING_EXT).await?;
            self.writer
                .write_u16(x.elements.len() as u16).await?;
            for b in x.elements.iter().map(|e| to_byte(e).unwrap()) {
                self.writer.write_u8(b).await?;
            }
        } else {
            if !x.is_nil() {
                self.writer.write_u8(LIST_EXT).await?;
                self.writer
                    .write_u32(x.elements.len() as u32).await?;
                for e in &x.elements {
                    self.encode_term(e).await?;
                }
            }
            self.encode_nil().await?;
        }
        Ok(())
    }
    async fn encode_improper_list(&mut self, x: &ImproperList) -> EncodeResult {
        self.writer.write_u8(LIST_EXT).await?;
        self.writer
            .write_u32(x.elements.len() as u32).await?;
        for e in &x.elements {
            self.encode_term(e).await?;
        }
        self.encode_term(&x.last).await?;
        Ok(())
    }
    async fn encode_tuple(&mut self, x: &Tuple) -> EncodeResult {
        if x.elements.len() < 0x100 {
            self.writer.write_u8(SMALL_TUPLE_EXT).await?;
            self.writer.write_u8(x.elements.len() as u8).await?;
        } else {
            self.writer.write_u8(LARGE_TUPLE_EXT).await?;
            self.writer
                .write_u32(x.elements.len() as u32).await?;
        }
        for e in &x.elements {
            self.encode_term(e).await?;
        }
        Ok(())
    }
    async fn encode_map(&mut self, x: &Map) -> EncodeResult {
        self.writer.write_u8(MAP_EXT).await?;
        self.writer.write_u32(x.map.len() as u32).await?;
        for (k, v) in x.map.iter() {
            self.encode_term(k).await?;
            self.encode_term(v).await?;
        }
        Ok(())
    }
    async fn encode_byte_list(&mut self, x: &[u8]) -> EncodeResult{
        self.writer.write_u8(STRING_EXT).await?;
        self.writer.write_u16(x.len() as u16).await?;
        self.writer.write_all(x).await?;
        
        Ok(())
    }
    async fn encode_binary(&mut self, x: &Binary) -> EncodeResult {
        self.writer.write_u8(BINARY_EXT).await?;
        self.writer.write_u32(x.bytes.len() as u32).await?;
        self.writer.write_all(&x.bytes).await?;
        Ok(())
    }
    async fn encode_bit_binary(&mut self, x: &BitBinary) -> EncodeResult {
        self.writer.write_u8(BIT_BINARY_EXT).await?;
        self.writer.write_u32(x.bytes.len() as u32).await?;
        self.writer.write_u8(x.tail_bits_size).await?;
        if !x.bytes.is_empty() {
            self.writer.write_all(&x.bytes[0..x.bytes.len() - 1]).await?;
            self.writer
                .write_u8(x.bytes[x.bytes.len() - 1] << (8 - x.tail_bits_size)).await?;
        }
        Ok(())
    }
    async fn encode_float(&mut self, x: &Float) -> EncodeResult {
        self.writer.write_u8(NEW_FLOAT_EXT).await?;
        self.writer.write_f64(x.value).await?;
        Ok(())
    }
    async fn encode_atom(&mut self, x: &Atom) -> EncodeResult {
        if x.name.len() > 0xFFFF {
            return Err(EncodeError::TooLongAtomName(x.clone()));
        }

        let is_ascii = x.name.as_bytes().iter().all(|&c| c < 0x80);
        if is_ascii {
            self.writer.write_u8(ATOM_EXT).await?;
        } else {
            self.writer.write_u8(ATOM_UTF8_EXT).await?;
        }
        self.writer.write_u16(x.name.len() as u16).await?;
        self.writer.write_all(x.name.as_bytes()).await?;
        Ok(())
    }
    async fn encode_fix_integer(&mut self, x: &FixInteger) -> EncodeResult {
        if 0 <= x.value && x.value <= i32::from(std::u8::MAX) {
            self.writer.write_u8(SMALL_INTEGER_EXT).await?;
            self.writer.write_u8(x.value as u8).await?;
        } else {
            self.writer.write_u8(INTEGER_EXT).await?;
            self.writer.write_i32(x.value as i32).await?;
        }
        Ok(())
    }
    async fn encode_big_integer(&mut self, x: &BigInteger) -> EncodeResult {
        let (sign, bytes) = x.value.to_bytes_le();

        if bytes.len() <= std::u8::MAX as usize {
            self.writer.write_u8(SMALL_BIG_EXT).await?;
            self.writer.write_u8(bytes.len() as u8).await?;
        } else if bytes.len() <= std::u32::MAX as usize {
            self.writer.write_u8(LARGE_BIG_EXT).await?;
            self.writer.write_u32(bytes.len() as u32).await?;
        } else {
            return Err(EncodeError::TooLargeInteger(x.clone()));
        }

        self.writer.write_u8(aux::sign_to_byte(sign)).await?;
        self.writer.write_all(&bytes).await?;
        Ok(())
    }
    async fn encode_pid(&mut self, x: &Pid) -> EncodeResult {
        self.writer.write_u8(NEW_PID_EXT).await?;
        self.encode_atom(&x.node).await?;
        self.writer.write_u32(x.id).await?;
        self.writer.write_u32(x.serial).await?;
        self.writer.write_u32(x.creation).await?;
        Ok(())
    }
    async fn encode_port(&mut self, x: &Port) -> EncodeResult {
        if (x.id >> 32) & 0xFFFFFFFF == 0 {
            self.writer.write_u8(NEW_PORT_EXT).await?;
            self.encode_atom(&x.node).await?;
            self.writer.write_u32(x.id as u32).await?;
            self.writer.write_u32(x.creation).await?;
        } else {
            self.writer.write_u8(V4_PORT_EXT).await?;
            self.encode_atom(&x.node).await?;
            self.writer.write_u64(x.id).await?;
            self.writer.write_u32(x.creation).await?;
        }
        Ok(())
    }
    async fn encode_reference(&mut self, x: &Reference) -> EncodeResult {
        self.writer.write_u8(NEWER_REFERENCE_EXT).await?;
        if x.id.len() > std::u16::MAX as usize {
            return Err(EncodeError::TooLargeReferenceId(x.clone()));
        }
        self.writer.write_u16(x.id.len() as u16).await?;
        self.encode_atom(&x.node).await?;
        self.writer.write_u32(x.creation).await?;
        for n in &x.id {
            self.writer.write_u32(*n).await?;
        }
        Ok(())
    }
    async fn encode_external_fun(&mut self, x: &ExternalFun) -> EncodeResult {
        self.writer.write_u8(EXPORT_EXT).await?;
        self.encode_atom(&x.module).await?;
        self.encode_atom(&x.function).await?;
        self.encode_fix_integer(&FixInteger::from(i32::from(x.arity))).await?;
        Ok(())
    }
    async fn encode_internal_fun(&mut self, x: &InternalFun) -> EncodeResult {
        match *x {
            InternalFun::Old {
                ref module,
                ref pid,
                ref free_vars,
                index,
                uniq,
            } => {
                self.writer.write_u8(FUN_EXT).await?;
                self.writer.write_u32(free_vars.len() as u32).await?;
                self.encode_pid(pid).await?;
                self.encode_atom(module).await?;
                self.encode_fix_integer(&FixInteger::from(index)).await?;
                self.encode_fix_integer(&FixInteger::from(uniq)).await?;
                for v in free_vars {
                    self.encode_term(v).await?;
                }
            }
            InternalFun::New {
                ref module,
                arity,
                ref pid,
                ref free_vars,
                index,
                ref uniq,
                old_index,
                old_uniq,
            } => {
                self.writer.write_u8(NEW_FUN_EXT).await?;

                let mut buf = Vec::new();
                {
                    let mut tmp = Encoder::new(&mut buf);
                    WriteBytesExt::write_u8(&mut tmp.writer, arity);
                    AsyncWriteExt::write_all(&mut tmp.writer, uniq);
                    WriteBytesExt::write_u32::<BigEndian>(&mut tmp.writer, index);
                    WriteBytesExt::write_u32::<BigEndian>(&mut tmp.writer, free_vars.len() as u32);
                    tmp.encode_atom(module);
                    tmp.encode_fix_integer(&FixInteger::from(old_index));
                    tmp.encode_fix_integer(&FixInteger::from(old_uniq));
                    tmp.encode_pid(pid);
                    for v in free_vars {
                        tmp.encode_term(v);
                    }
                }
                self.writer.write_u32(4 + buf.len() as u32).await?;
                self.writer.write_all(&buf).await?;
            }
        }
        Ok(())
    }
}

