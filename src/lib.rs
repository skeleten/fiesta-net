#![allow(dead_code)]
#![allow(unused_variables)]
#![feature(convert)]

#[macro_use]
extern crate log;
extern crate mio;
extern crate chan;
extern crate threadpool;

mod buffer;
mod client;
mod processing;

#[test]
fn it_works() {
}
