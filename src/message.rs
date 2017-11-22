// Copyright (c) 2017 Ivo Wetzel

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


// STD Dependencies -----------------------------------------------------------
use std::marker::PhantomData;


// External Dependencies ------------------------------------------------------
use serde::Serialize;
use serde::de::DeserializeOwned;
use bincode::{serialized_size, deserialize};


// Traits ---------------------------------------------------------------------
pub trait Message: Serialize + DeserializeOwned {}


// Internal Messages ----------------------------------------------------------
#[derive(Debug, Serialize, Deserialize)]
pub enum InternalMessage {
    Ping(u8, u64),
    Pong(u8, u64, u64)
}


// Message Iterator Abstraction -----------------------------------------------
pub struct MessageIterator<'a, M: Serialize + DeserializeOwned, I: Serialize + DeserializeOwned + 'a> {
    buffer: &'a mut Vec<u8>,
    internal_queue: &'a mut Vec<I>,
    message: PhantomData<M>
}

fn from_bytes<M: Serialize + DeserializeOwned>(bytes: &[u8]) -> Option<(M, usize)> {
    if let Ok(msg) = deserialize::<M>(bytes) {
        let len = serialized_size(&msg) as usize;
        Some((msg, len))

    } else {
        None
    }
}

impl<'a, M: Serialize + DeserializeOwned, I: Serialize + DeserializeOwned> Iterator for MessageIterator<'a, M, I> {

    type Item = M;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buffer.is_empty() {
            None

        } else {
            let mut index = 0;
            let mut message = None;

            while index < self.buffer.len() {

                // Internal Messages
                if self.buffer[index] == 0 {
                    if let Some((msg, bytes)) = from_bytes::<I>(&self.buffer[index + 1..]) {
                        self.internal_queue.push(msg);
                        index += bytes;
                    }

                // Application Messages
                } else if self.buffer[index] == 1 {
                    if let Some((msg, bytes)) = from_bytes::<M>(&self.buffer[index + 1..]) {
                        message = Some(msg);
                        index += bytes + 1;
                        break;
                    }
                }

                index += 1;

            }

            *self.buffer = (&self.buffer[index..]).to_vec();
            message
        }
    }

}


// Internal Factory -----------------------------------------------------------
pub fn create_message_iterator<'a, M: Serialize + DeserializeOwned, I: Serialize + DeserializeOwned>(
    buffer: &'a mut Vec<u8>,
    internal_queue: &'a mut Vec<I>

) -> MessageIterator<'a, M, I> {
    MessageIterator {
        buffer: buffer,
        internal_queue: internal_queue,
        message: PhantomData
    }
}

