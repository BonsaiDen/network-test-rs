// Copyright (c) 2017 Ivo Wetzel

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


// Crates ---------------------------------------------------------------------
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate bincode;


// Modules --------------------------------------------------------------------
mod client;
mod message;
mod protocol;
mod server;
mod time;


// Exports --------------------------------------------------------------------
pub use self::client::Client;
pub use self::protocol::TCP;
pub use self::server::{Remote, Server};
pub use self::message::{Message, MessageIterator};

