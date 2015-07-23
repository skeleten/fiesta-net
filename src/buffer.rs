use std::io::{Error, ErrorKind, Write};
use mio::buf::*;

/* buffer of clients */
pub const BUFFERSIZE: usize = 65000;

pub trait BinaryReadable {
	fn read_bytes(&mut self, size: usize) -> Result<Vec<u8>, Error>;
	fn read_u8(&mut self) -> Result<u8, Error> {
		let buf = try!(self.read_bytes(1));
		let result = buf[0];

		Ok(result)
	}
	fn read_i8(&mut self) -> Result<i8, Error> {
		let buf = try!(self.read_bytes(1));
		let result = buf[0] as i8;

		Ok(result)
	}
	fn read_u16(&mut self) -> Result<u16, Error> {
		let buf = try!(self.read_bytes(2));
		let result = 
				(buf[1] as u16) 
			|	((buf[0] as u16) << 8);

		Ok(result)
	}
	fn read_i16(&mut self) -> Result<i16, Error> {
		let buf = try!(self.read_bytes(2));
		let result = 
				(buf[1] as i16) 
			|	((buf[0] as i16) << 8);

		Ok(result)
	}
	fn read_u32(&mut self) -> Result<u32, Error> {
		let buf = try!(self.read_bytes(4));
		let result = 
				(buf[3] as u32)
			|	((buf[2] as u32) << 8)
			|	((buf[1] as u32) << 16)
			|	((buf[0] as u32) << 24);

		Ok(result)
	}
	fn read_i32(&mut self) -> Result<i32, Error> {
		let buf = try!(self.read_bytes(4));
		let result = 
				(buf[3] as i32)
			|	((buf[2] as i32) << 8)
			|	((buf[1] as i32) << 16)
			|	((buf[0] as i32) << 24);

		Ok(result)
	}
	fn read_u64(&mut self) -> Result<u64, Error> {
		let buf = try!(self.read_bytes(8));
		let result = 
				(buf[7] as u64)
			|	((buf[6] as u64) << 8)
			|	((buf[5] as u64) << 16)
			|	((buf[4] as u64) << 24)
			|	((buf[3] as u64) << 32)
			|	((buf[2] as u64) << 40)
			|	((buf[1] as u64) << 48)
			|	((buf[0] as u64) << 56);

		Ok(result)
	}
	fn read_i64(&mut self) -> Result<i64, Error> {
		let buf = try!(self.read_bytes(8));
		let result = 
				(buf[7] as i64)
			|	((buf[6] as i64) << 8)
			|	((buf[5] as i64) << 16)
			|	((buf[4] as i64) << 24)
			|	((buf[3] as i64) << 32)
			|	((buf[2] as i64) << 40)
			|	((buf[1] as i64) << 48)
			|	((buf[0] as i64) << 56);

		Ok(result)
	}
}

pub trait BinaryPeekable {
	fn peek_bytes(&mut self, offset: usize, size: usize) -> Result<Vec<u8>, Error>;

	fn peek_u8(&mut self, offset: usize) -> Result<u8, Error> {
		let buf = try!(self.peek_bytes(offset, 1));
		let result = buf[0];

		Ok(result)
	}
	fn peek_i8(&mut self, offset: usize) -> Result<i8, Error> {
		let buf = try!(self.peek_bytes(offset, 1));
		let result = buf[0] as i8;

		Ok(result)
	}
	fn peek_u16(&mut self, offset: usize) -> Result<u16, Error> {
		let buf = try!(self.peek_bytes(offset, 2));
		let result = 
				(buf[1] as u16) 
			|	((buf[0] as u16) << 8);

		Ok(result)
	}
	fn peek_i16(&mut self, offset: usize) -> Result<i16, Error> {
		let buf = try!(self.peek_bytes(offset, 2));
		let result = 
				(buf[1] as i16) 
			|	((buf[0] as i16) << 8);

		Ok(result)
	}
	fn peek_u32(&mut self, offset: usize) -> Result<u32, Error> {
		let buf = try!(self.peek_bytes(offset, 4));
		let result = 
				(buf[3] as u32)
			|	((buf[2] as u32) << 8)
			|	((buf[1] as u32) << 16)
			|	((buf[0] as u32) << 24);

		Ok(result)
	}
	fn peek_i32(&mut self, offset: usize) -> Result<i32, Error> {
		let buf = try!(self.peek_bytes(offset, 4));
		let result = 
				(buf[3] as i32)
			|	((buf[2] as i32) << 8)
			|	((buf[1] as i32) << 16)
			|	((buf[0] as i32) << 24);

		Ok(result)
	}
	fn peek_u64(&mut self, offset: usize) -> Result<u64, Error> {
		let buf = try!(self.peek_bytes(offset, 8));
		let result = 
				(buf[7] as u64)
			|	((buf[6] as u64) << 8)
			|	((buf[5] as u64) << 16)
			|	((buf[4] as u64) << 24)
			|	((buf[3] as u64) << 32)
			|	((buf[2] as u64) << 40)
			|	((buf[1] as u64) << 48)
			|	((buf[0] as u64) << 56);

		Ok(result)
	}
	fn peek_i64(&mut self, offset: usize) -> Result<i64, Error> {
		let buf = try!(self.peek_bytes(offset, 8));
		let result = 
				(buf[7] as i64)
			|	((buf[6] as i64) << 8)
			|	((buf[5] as i64) << 16)
			|	((buf[4] as i64) << 24)
			|	((buf[3] as i64) << 32)
			|	((buf[2] as i64) << 40)
			|	((buf[1] as i64) << 48)
			|	((buf[0] as i64) << 56);

		Ok(result)
	}
}

pub struct Buffer {
	buffer:			RingBuf,
	remaining:		usize,
}

impl Buffer {
	pub fn new() -> Self {
		Buffer {
			buffer:		RingBuf::new(BUFFERSIZE),
			remaining:	0,
		}
	}

	pub fn bytes_remaining(&self) -> usize {
		self.remaining
	}

	pub fn append(&mut self, bytes: &[u8]) {
		self.buffer.write(bytes).unwrap();
		self.remaining += bytes.len();
		if self.remaining > BUFFERSIZE {
			// panic!("overwritten some data!!");
			warn!(target: "networking", "overwritten some data.");
			self.remaining = BUFFERSIZE;
		}
	}

	pub fn advance_read(&mut self, bytes: usize) {
		<RingBuf as Buf>::advance(&mut self.buffer, bytes);
		self.remaining -= bytes;
	}

	pub fn peek_max(&mut self, offset: usize, len: usize, buf: &mut [u8]) -> Result<usize, Error> {
		self.buffer.mark();
		<RingBuf as Buf>::advance(&mut self.buffer, offset);
		let size = self.buffer.read_slice(buf);
		self.buffer.reset();
		Ok(size)
	}
}

impl BinaryReadable for Buffer {
	fn read_bytes(&mut self, size: usize) -> Result<Vec<u8>, Error> {
		if self.bytes_remaining() < size {
			Err(Error::new(ErrorKind::InvalidData, "Not enough data"))
		} else {
			let mut buf = vec![0; size];

			if <RingBuf as Buf>::read_slice(&mut self.buffer, &mut buf[..]) != size {
				Err(Error::new(ErrorKind::InvalidData, "Didn't read as many bytes as specified!"))
			} else {
				self.remaining -= size;
				Ok(buf)
			}
		}

	}
}

impl BinaryPeekable for Buffer {
	fn peek_bytes(&mut self, offset: usize, size: usize) -> Result<Vec<u8>, Error> {
		if <RingBuf as Buf>::remaining(&self.buffer) < size + offset {
			Err(Error::new(ErrorKind::InvalidData, "Not enough data"))
		} else {
			self.buffer.mark();

			let mut buf = vec![0; size];
			<RingBuf as Buf>::advance(&mut self.buffer, offset);
			if <RingBuf as Buf>::read_slice(&mut self.buffer, buf.as_mut_slice()) != size {
				self.buffer.reset();
				Err(Error::new(ErrorKind::InvalidData, "Didn't read as many bytes as specified!"))
			} else {
				self.buffer.reset();
				Ok(buf)
			}
		}
	}
}