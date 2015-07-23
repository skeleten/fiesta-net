use std::thread::{JoinHandle, Builder};
use std::sync::{Arc, RwLock};
use chan::{Receiver, Sender, async};
use client;
use client::*;

pub trait PacketProcessor: Send + 'static {
	fn process_packet(&mut self, info: Arc<RwLock<Box<PacketProcessingInfo>>>);
	fn clone(&self) -> Box<PacketProcessor>;
}

pub struct PacketProcessingThreadPool {
	thread_handles:					Arc<RwLock<Vec<JoinHandle<()>>>>,
	packet_receiver:				Receiver<Arc<RwLock<Box<PacketProcessingInfo>>>>,
	packet_sender:					Sender<Arc<RwLock<Box<PacketProcessingInfo>>>>,
	processor:						Box<PacketProcessor>,
}

pub struct PacketProcessingInfo {
	pub packet:			Arc<RwLock<FiestaPacket>>,
	pub client:			Arc<RwLock<Box<FiestaNetworkClient>>>,
}

impl PacketProcessingInfo {
	pub fn new(packet: FiestaPacket, client: Arc<RwLock<Box<FiestaNetworkClient>>>) -> Self {
		PacketProcessingInfo {
			packet:		Arc::new(RwLock::new(packet)),
			client:		client.clone(),
		}
	}
}

// a test
// unsafe impl Send for PacketProcessingInfo { }

impl PacketProcessingThreadPool {
	pub fn new(threads: usize, processor: Box<PacketProcessor>) -> PacketProcessingThreadPool {
		let (s, r) = async();

		let mut result = PacketProcessingThreadPool {
			thread_handles:				Arc::new(RwLock::new(Vec::with_capacity(threads))),
			packet_receiver:			r,
			packet_sender:				s,
			processor:					processor.clone(),
		};
		for i in 0..threads {
			result.start_new_thread(i);
			debug!(target: "threading", "started packet processing thread {}", i);
		};

		result
	}

	pub fn start_new_thread(&mut self, id: usize) {
		let rec = self.packet_receiver.clone();
		let mut processor = self.processor.clone();

		let handle = Builder::new()
			.name(format!("WRKR {}", id))
			.spawn(move || {
				for packet in rec.iter() {
					processor.process_packet(packet);
				}
			}).unwrap();
		let mut handles = self.thread_handles.write().unwrap();
		handles.push(handle);
	}
}

impl Clone for PacketProcessingThreadPool {
	fn clone(&self) -> Self {
		PacketProcessingThreadPool {
			thread_handles:			self.thread_handles.clone(),
			packet_receiver:		self.packet_receiver.clone(),
			packet_sender:			self.packet_sender.clone(),
			processor:				self.processor.clone(),
		}
	}
} 

impl PacketProcessor for PacketProcessingThreadPool {
	fn process_packet(&mut self, info: Arc<RwLock<Box<PacketProcessingInfo>>>) {
		self.packet_sender.send(info);
	}

	fn clone(&self) -> Box<PacketProcessor> {
		Box::new(<PacketProcessingThreadPool as Clone>::clone(&self))
	}
}