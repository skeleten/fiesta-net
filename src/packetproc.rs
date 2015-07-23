use std::thread::{JoinHandle, Builder};
use std::sync::{Arc, RwLock, Mutex};
use std::sync::mpsc::{Sender, Receiver, channel};
use client::{FiestaNetworkClient, FiestaPacket};
use threadpool::ThreadPool;

pub trait PacketProcessor: Send + 'static {
	fn process_packet(&mut self, info: Arc<RwLock<Box<PacketProcessingInfo>>>);
	fn clone(&self) -> Box<PacketProcessor>;
}

pub struct PacketProcessingThreadPool {
	pub processor:						Box<PacketProcessor>,
	pub threadpool:						Arc<Mutex<Box<ThreadPool>>>,
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
		let mut result = PacketProcessingThreadPool {
			processor:					processor.clone(),
			threadpool:					Arc::new(Mutex::new(Box::new(ThreadPool::new(threads)))),
		};

		result
	}

}

impl Clone for PacketProcessingThreadPool {
	fn clone(&self) -> Self {
		PacketProcessingThreadPool {
			processor:				self.processor.clone(),
			threadpool:				self.threadpool.clone(),
		}
	}
} 

impl PacketProcessor for PacketProcessingThreadPool {
	fn process_packet(&mut self, info: Arc<RwLock<Box<PacketProcessingInfo>>>) {
		// todo..
		let mut processor = self.processor.clone();
		let threadpool = self.threadpool.lock().unwrap();
		threadpool.execute(move || processor.process_packet(info));
	}

	fn clone(&self) -> Box<PacketProcessor> {
		Box::new(<PacketProcessingThreadPool as Clone>::clone(&self))
	}
}