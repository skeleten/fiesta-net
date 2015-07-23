use std::sync::{ 
	Arc, 
	Mutex, 
	RwLock
};

use super::packetproc::*;

pub trait PacketProcessor: Send + 'static {
	fn process_packet(&mut self, info: Arc<RwLock<Box<PacketProcessingInfo>>>);
	fn clone(&self) -> Box<PacketProcessor>;
}