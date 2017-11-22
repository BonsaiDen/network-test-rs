// Copyright (c) 2017 Ivo Wetzel

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


// STD Dependencies -----------------------------------------------------------
use std::time::Duration;
use std::marker::PhantomData;
use std::net::{SocketAddr, ToSocketAddrs};
use std::io::{Error as IOError, ErrorKind};


// External Dependencies ------------------------------------------------------
use serde::Serialize;
use serde::de::DeserializeOwned;
use bincode::{serialize, Infinite};


// Internal Dependencies ------------------------------------------------------
use ::time::Timer;
use ::protocol::{Protocol, Connection};
use ::message::{MessageIterator, InternalMessage, create_message_iterator};


// Client Abstraction ---------------------------------------------------------
pub struct Client<P: Protocol, M: Serialize + DeserializeOwned> {
    connection: Option<P::Connection>,
    incoming: Vec<u8>,
    internal_messages: Vec<InternalMessage>,
    timer: Timer,
    message: PhantomData<M>
}

impl<P: Protocol, M: Serialize + DeserializeOwned> Client<P, M> {

    pub fn new(ticks_per_second: u8) -> Self {
        Self {
            connection: None,
            incoming: Vec::new(),
            internal_messages: Vec::new(),
            timer: Timer::new(ticks_per_second),
            message: PhantomData
        }
    }

    pub fn rtt(&self) -> f64 {
        self.timer.rtt()
    }

    pub fn clock(&self) -> f64 {
        self.timer.clock()
    }

    pub fn peer_addr(&self) -> Result<SocketAddr, IOError> {
        if let Some(connection) = self.connection.as_ref() {
            connection.peer_addr()

        } else {
            Err(IOError::new(ErrorKind::NotConnected, ""))
        }
    }

    pub fn connect<A: ToSocketAddrs>(&mut self, addr: A, timeout: Duration) -> Result<(), IOError> {
        if self.connection.is_none() {
            let connection = P::Connection::connect(addr, timeout)?;
            self.connection = Some(connection);
            self.timer.reset();
            Ok(())

        } else {
            Err(IOError::new(ErrorKind::AlreadyExists, ""))
        }
    }

    pub fn send(&mut self, message: M) -> Result<(), IOError> {
        self.send_raw(1, message)
    }

    pub fn receive(&mut self) -> Result<MessageIterator<M, InternalMessage>, IOError> {
        if let Some(connection) = self.connection.as_mut() {
            connection.read(&mut self.incoming)?;
            Ok(create_message_iterator(&mut self.incoming, &mut self.internal_messages))

        } else {
            Err(IOError::new(ErrorKind::NotConnected, ""))
        }
    }

    pub fn sleep(&mut self) {
        let messages = self.internal_messages.drain(0..).collect::<Vec<_>>();
        for m in self.timer.receive(messages) {
            self.send_raw(0, m).ok();
        }
        self.timer.sleep();
    }

    pub fn disconnect(&mut self) -> Result<(), IOError> {
        if let Some(mut connection) = self.connection.take() {
            connection.shutdown()

        } else {
            Err(IOError::new(ErrorKind::NotConnected, ""))
        }
    }


    // Internal ---------------------------------------------------------------
    fn send_raw<T: Serialize + DeserializeOwned>(&mut self, prefix: u8, message: T) -> Result<(), IOError> {
        if let Some(connection) = self.connection.as_mut() {
            if let Ok(message_bytes) = serialize(&message, Infinite) {
                let mut bytes = vec![prefix];
                bytes.extend(message_bytes);
                connection.write(&bytes[..])?;
                Ok(())

            } else {
                Err(IOError::new(ErrorKind::InvalidData, ""))
            }

        } else {
            Err(IOError::new(ErrorKind::NotConnected, ""))
        }
    }

}

