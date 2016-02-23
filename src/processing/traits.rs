use std::sync::{
	Arc,
	RwLock
};

use super::packetproc::PacketProcessingInfo;

pub trait PacketProcessor: Send + 'static {
	fn process_packet(&mut self, info: Arc<RwLock<Box<PacketProcessingInfo>>>);
	fn clone(&self) -> Box<PacketProcessor>;
}
