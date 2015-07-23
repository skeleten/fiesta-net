use std::collections::{HashMap, LinkedList};
use std::io::{Error, ErrorKind, Read, Write};
use std::sync::{Mutex, Arc, RwLock, MutexGuard};
use std::mem::drop;
use mio::*;
use mio::tcp::*;

use buffer::*;
use super::processing::*;

pub const SERVER_TOKEN: Token = Token(0);

pub struct FiestaHandler {
	listener:		TcpListener,
	clients:		HashMap<Token, Arc<RwLock<Box<FiestaNetworkClient>>>>,
	token_count:	usize,
	processor:		Box<PacketProcessor>,
}

pub struct FiestaNetworkClient {
	client:			Mutex<TcpStream>,
	read_buffer:	Mutex<Buffer>,
	write_buffer:	Mutex<Buffer>,
	packet_queue:	Mutex<LinkedList<FiestaPacket>>,
	is_alive:		Mutex<bool>,
	interest:		Mutex<EventSet>,
	id:				Token,
}

pub struct FiestaPacket {
	pub header:			u16,
	pub data:			Buffer,
}

impl FiestaNetworkClient {
	pub fn new(inner_client: TcpStream, id: Token) -> Self {
		FiestaNetworkClient {
			client:			Mutex::new(inner_client),
			read_buffer:	Mutex::new(Buffer::new()),
			write_buffer:	Mutex::new(Buffer::new()),
			packet_queue:	Mutex::new(LinkedList::new()),
			is_alive:		Mutex::new(true),
			interest:		Mutex::new(EventSet::all()),
			id:				id
		}
	}

	pub fn can_read_next_packet(&self) -> bool {
		match self.get_next_size() {
			Ok(s) => {
				let guard = self.read_buffer.lock().unwrap();
				guard.bytes_remaining() >= (s as usize) + 2 + if s > 255 { 2 } else { 1 }
				/* for the size of the header/size info */
			},
			Err(_) => false
		}
	}

	fn can_read_next_packet_inner(guard: &mut MutexGuard<Buffer>) -> bool {
		match FiestaNetworkClient::get_next_size_inner(guard) {
			Ok(s) => {
				let total_size =
						s
					+	2	/* header */
					+	if s > 255 { 2 } else { 1 };	/* size data */

				guard.bytes_remaining() >= total_size as usize
			},
			Err(_) => false,
		}
	}

	pub fn read_next_packet(&self) {
		let mut read_buffer_guard = self.read_buffer.lock().unwrap();
		let mut packet_queue_guard = self.packet_queue.lock().unwrap();

		FiestaNetworkClient::read_next_packet_inner(&mut read_buffer_guard, &mut packet_queue_guard);
	}

	fn read_next_packet_inner(
			read_buffer: &mut MutexGuard<Buffer>, 
			packet_queue: &mut MutexGuard<LinkedList<FiestaPacket>>) {

		if FiestaNetworkClient::can_read_next_packet_inner(read_buffer) {
			let size = match FiestaNetworkClient::get_next_size_inner(read_buffer) {
				Ok(s) => s,
				Err(_) => return,
			};
			let mut packet = FiestaPacket::new(0, size as usize);

			if size > 255 {
				read_buffer.advance_read(3);
			} else {
				read_buffer.advance_read(1);
			};

			packet.header = read_buffer.read_u16().unwrap();
			let body = read_buffer.read_bytes(size as usize).unwrap();
			packet.data.append(&body[..]);
			packet_queue.push_back(packet);
		}
	}

	fn get_next_size(&self) -> Result<u16, Error> {
		let mut guard = self.read_buffer.lock().unwrap();
		FiestaNetworkClient::get_next_size_inner(&mut guard)
	}

	fn get_next_size_inner(guard: &mut MutexGuard<Buffer>) -> Result<u16, Error> {
		if guard.bytes_remaining() < 3 {
			Err(Error::new(ErrorKind::Other, "to little data left"))
		} else {
			let small_size = try!(guard.peek_u8(0));
			if small_size > 0 {
				Ok(small_size as u16)
			} else {
				let mut big_size = try!(guard.peek_u16(1));

				if (big_size as usize) > 2048 {
					/* this should never actually happen with real data */
					/* casting 0 here will still let it read 5 bytes (size + header) */
					big_size = 0;
				};

				Ok(big_size)
			}
		}
	}

	pub fn readable(&self, event_loop: &mut EventLoop<FiestaHandler>, token: Token, disconnect: &mut bool) {
		let mut buffer = [0; 2048]; /* maybe not allocate this every time again? */
		let mut inner_client_guard = self.client.lock().unwrap();
		let mut read_buffer_guard = self.read_buffer.lock().unwrap();

		match inner_client_guard.read(&mut buffer[..]) {
			Ok(size) if size > 0 => {
				/* read some data */
				info!(target: "network", "read {} bytes from {:?}", size, token);
				read_buffer_guard.append(&buffer[0..size]);
			},
			Ok(_) => {
				/* size == 0 */
				debug!(target: "network", "read 0 bytes from {:?}", self.id());
				/* this usually means a disconect */
				/* no need to deregister, we use oneshot. */
				// event_loop.deregister(&*inner_client_guard).unwrap();
				inner_client_guard.shutdown(Shutdown::Both).unwrap();
				self.set_alive(false);
				*disconnect = true;
			},
			Err(e) => {
				/* some error while receiving data.. */
				warn!(target: "network", "error while receiving data: '{:#?}'", e);
				/* no need to deregister, we use oneshot. */
				// event_loop.deregister(&*inner_client_guard);
				inner_client_guard.shutdown(Shutdown::Both).unwrap();
				self.set_alive(false);
				*disconnect = true;
			}
		}

		/* this is no longer needed, as it is a mutex, I like to drop it ASAP */
		drop(inner_client_guard);
		
		let mut packet_queue_guard = self.packet_queue.lock().unwrap();
		while FiestaNetworkClient::can_read_next_packet_inner(&mut read_buffer_guard) {
			FiestaNetworkClient::read_next_packet_inner(&mut read_buffer_guard, &mut packet_queue_guard);
		}
	}

	pub fn writeable(&self, event_loop: &mut EventLoop<FiestaHandler>, token: Token, disconnect: &mut bool) {
		let mut buf = [0; 1024];
		let mut guard = self.write_buffer.lock().unwrap();
		match guard.peek_max(0, 1024, &mut buf[..]) {
			Ok(size) if size > 0	=> {
				let mut inner_client_guard = self.client.lock().unwrap();
				match inner_client_guard.write(&buf[0..size]) {
					Ok(s) if s > 0 => {
						debug!(target: "network", "wrote {} bytes to {:?}", s, token);
						guard.advance_read(s);
					},
					Ok(_) => {
						/* size == 0 */
						warn!(target: "network", "wrote 0 bytes for {:?}, shutting down the socket.", token);
						/* no need to deregister, we use oneshot. */
						inner_client_guard.shutdown(Shutdown::Both).unwrap();
						self.set_alive(false);
						*disconnect = true;
					},
					Err(e) => {
						/* error while writing */
						warn!(target: "network", "error while writing to socket ({:?}): {:#?}", token, e);
						/* no need to deregister, we use oneshot. */
						inner_client_guard.shutdown(Shutdown::Both).unwrap();
						self.set_alive(false);
						*disconnect = true;
					}
				}
			},
			Ok(_)	=> {
				/* read 0 bytes from send buffer..  */
				/* TODO: we might want to unregister it from the loop until new data arrives */
				let interest = self.interest();
				let interest = interest ^ EventSet::writable();
				self.set_interest(interest);
			},
			Err(e)		=> {
				warn!(target: "network", "error while reading from write_buffer ({:?}): {:#?}", token, e);
				let inner_client_guard = self.client.lock().unwrap();
				/* no need to deregister, we use oneshot */
				inner_client_guard.shutdown(Shutdown::Both).unwrap();
				*disconnect = true;
			}
		};
	}

	pub fn alive(&self) -> bool {
		let guard = self.is_alive.lock().unwrap();
		(*guard).clone()
	}

	pub fn id(&self) -> Token {
		self.id
	}

	fn set_alive(&self, value: bool) {
		let mut guard = self.is_alive.lock().unwrap();
		*guard = value;
	}

	pub fn interest(&self) -> EventSet {
		let guard = self.interest.lock().unwrap();
		(*guard).clone()
	}

	fn set_interest(&self, interest: EventSet) {
		let mut guard = self.interest.lock().unwrap();
		*guard = interest;
	}

	pub fn append_send(&self, buffer: &[u8]) {
		let mut guard = self.write_buffer.lock().unwrap();
		guard.append(buffer);
		let mut interest_guard = self.interest.lock().unwrap();
		if !interest_guard.is_writable() {
			*interest_guard = (*interest_guard) | EventSet::writable();
		}
	}
}

impl FiestaHandler {
	pub fn new(listener: TcpListener, processor: Box<PacketProcessor>) -> FiestaHandler {
		FiestaHandler {
			listener:			listener,
			clients:			HashMap::new(),
			token_count:		0,
			processor:			processor,
		}
	}

	fn server_ready(&mut self, event_loop: &mut EventLoop<Self>, token: Token, events: EventSet) {
		if events.is_readable() {
			/* we may accept a client */
			match self.listener.accept() {
				Ok(Some(client)) => {
					/* successfully accepted a client */
					let token = self.get_next_token();
					event_loop.register_opt(&client, token, EventSet::all(), PollOpt::oneshot()).unwrap();
					self.clients.insert(
						token, 
						Arc::new(
							RwLock::new(
								Box::new(
									FiestaNetworkClient::new(client, token)))));
					info!(target: "network", "accepted client with {:?}", token);
				},
				Ok(None) => {
					/* WOULDBLOCK / EAGAIN */
					info!(target: "network", "WOULDBLOCK while accepting client.");
				},
				Err(e) => {
					/* unexpected error */
					panic!("unexpected error: {:#?}", e);
				}
			}
		}
	}

	fn get_next_token(&mut self) -> Token {
		self.token_count += 1;
		Token(self.token_count)
	}

	fn get_current_token(&self) -> Token {
		Token(self.token_count)
	}

	fn client_ready(&mut self, event_loop: &mut EventLoop<Self>, token: Token, events: EventSet) {
		let mut client_disconnect = false;
		let mut packets_to_process = Vec::new();

		if events.is_readable() {
			let client = self.clients.get(&token).unwrap();
			let client_guard = client.read().unwrap();
			client_guard.readable(event_loop, token, &mut client_disconnect);

			let mut packet_queue_guard = client_guard.packet_queue.lock().unwrap();
			while !packet_queue_guard.is_empty() {
				let packet = packet_queue_guard.pop_front().unwrap();
				packets_to_process.push(
					Arc::new(
						RwLock::new(
							Box::new(
								PacketProcessingInfo::new(
									packet,
									client.clone())))));
			}
		}

		if events.is_writable() {
			let client = self.clients.get_mut(&token).unwrap();
			let guard = client.read().unwrap();
			guard.writeable(event_loop, token, &mut client_disconnect);
		}

		for packet in packets_to_process.into_iter() {
			self.processor.process_packet(packet);
		};

		/* we need to have this down here, because of borrows.. */
		if client_disconnect {
			self.clients.remove(&token);
			info!(target: "network", "client {:?} disconnected.", token);
		} else {
			/* re-register */
			let client = self.clients.get(&token).unwrap();
			let client_borrow = client.read().unwrap();
			let inner_client_guard = client_borrow.client.lock().unwrap();
			let interest = client_borrow.interest();
			event_loop.reregister(&*inner_client_guard, token, interest, PollOpt::oneshot()).unwrap();
		}
	}
}

impl Handler for FiestaHandler {
	type Timeout = usize;
	type Message = ();

	fn ready(&mut self, event_loop: &mut EventLoop<Self>, token: Token, events: EventSet) {
		match token {
			SERVER_TOKEN 	=> self.server_ready(event_loop, SERVER_TOKEN, events),
			t 				=> self.client_ready(event_loop, t, events),
		}
	}
}

impl FiestaPacket {
	pub fn new(header: u16, size: usize) -> Self {
		FiestaPacket {
			header:			header,
			data:			Buffer::with_capacity(size),
		}
	}
}