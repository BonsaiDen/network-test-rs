// Copyright (c) 2017 Ivo Wetzel

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


// STD Dependencies -----------------------------------------------------------
use std::marker::PhantomData;
use std::io::{Error as IOError, ErrorKind};
use std::net::{SocketAddr, ToSocketAddrs};


// External Dependencies ------------------------------------------------------
use serde::Serialize;
use serde::de::DeserializeOwned;
use bincode::{serialize, Infinite};


// Internal Dependencies ------------------------------------------------------
use ::time::Timer;
use ::protocol::{Protocol, Connection, Host};
use ::message::{MessageIterator, InternalMessage, create_message_iterator};


// Server Abstraction ---------------------------------------------------------
pub struct Server<P: Protocol, M: Serialize + DeserializeOwned, D> {
    listener: Option<P::Host>,
    remotes: Vec<(Remote<<<P as Protocol>::Host as Host>::Connection, M>, D)>,
    closed_indexes: Vec<usize>,
    timer: Timer,
    accepted_done: bool,
    connected_done: bool,
    closed_done: bool
}

impl<P: Protocol, M: Serialize + DeserializeOwned, D> Server<P, M, D> {

    pub fn new(ticks_per_second: u8) -> Self {
        Self {
            listener: None,
            timer: Timer::new(ticks_per_second),
            remotes: Vec::new(),
            closed_indexes: Vec::new(),
            accepted_done: false,
            connected_done: false,
            closed_done: false
        }
    }

    pub fn bind<A: ToSocketAddrs>(&mut self, addr: A) -> Result<(), IOError> {
        if self.listener.is_none() {
            let listener = P::Host::bind(addr)?;
            self.listener = Some(listener);
            self.timer.reset();
            Ok(())

        } else {
            Err(IOError::new(ErrorKind::AlreadyExists, ""))
        }
    }

    pub fn accepted_with<'a, C: FnMut(SocketAddr) -> Option<D>>(&'a mut self, mut data: C) -> Box<Iterator<Item=&mut (Remote<<<P as Protocol>::Host as Host>::Connection, M>, D)> + 'a> {

        if !self.accepted_done {

            self.accepted_done = true;

            // Accept new connections
            if let Some(listener) = self.listener.as_mut() {
                while let Ok(mut connection) = listener.accept() {
                    if let Some(data) = data(connection.peer_addr().unwrap()) {
                        self.remotes.push((Remote::from_connection(
                            connection,
                            self.timer.clone()

                        ), data));

                    } else {
                        connection.shutdown().ok();
                    }
                }
            }

        }

        Box::new(self.remotes.iter_mut().filter(|entry| entry.0.accepted() ))

    }

    pub fn connected<'a>(&'a mut self) -> Box<Iterator<Item=&mut (Remote<<<P as Protocol>::Host as Host>::Connection, M>, D)> + 'a> {

        if !self.connected_done {

            self.connected_done = true;

            for &mut (ref mut remote, _) in &mut self.remotes {
                if self.listener.is_some() {
                    remote.read();
                }
            }

        }

        Box::new(self.remotes.iter_mut().filter(|remote| remote.0.connected() ))

    }

    pub fn closed<'a>(&'a mut self) -> Box<Iterator<Item=(Remote<<<P as Protocol>::Host as Host>::Connection, M>, D)> + 'a> {

        if !self.closed_done {
            self.closed_done = true;
            for (index, &mut (ref mut remote, _)) in self.remotes.iter_mut().enumerate() {
                remote.write();
                if remote.closed() {
                    self.closed_indexes.push(index);
                }
            }
        }

        let mut closed = Vec::new();
        for (offset, index) in self.closed_indexes.iter().enumerate() {
            closed.push(self.remotes.swap_remove(index - offset));
        }
        self.closed_indexes.clear();

        Box::new(closed.into_iter())

    }

    pub fn sleep(&mut self) {
        self.accepted_done = false;
        self.connected_done = false;
        self.closed_done = false;
        self.timer.sleep();
    }

    pub fn shutdown(&mut self) -> Result<(), IOError> {
        if self.listener.take().is_some() {
            for &mut (ref mut remote, _) in &mut self.remotes {
                remote.close().ok();
                remote.try_close();
            }
            self.closed_indexes.clear();
            self.remotes.clear();
            Ok(())

        } else {
            Err(IOError::new(ErrorKind::NotConnected, ""))
        }
    }

}

#[derive(Eq, PartialEq)]
enum RemoteState {
    Accepted,
    Connected,
    Closing,
    Closed
}

pub struct Remote<C: Connection, M: Serialize + DeserializeOwned> {
    connection: C,
    incoming: Vec<u8>,
    outgoing: Vec<u8>,
    internal_messages: Vec<InternalMessage>,
    timer: Timer,
    state: RemoteState,
    message: PhantomData<M>
}

impl<C: Connection, M: Serialize + DeserializeOwned> Remote<C, M> {

    pub fn rtt(&self) -> f64 {
        self.timer.rtt()
    }

    pub fn clock(&self) -> f64 {
        self.timer.clock()
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.connection.peer_addr().unwrap()
    }

    pub fn send(&mut self, message: M) {
        self.send_raw(1, message)
    }

    pub fn receive(&mut self) -> MessageIterator<M, InternalMessage> {
        create_message_iterator(&mut self.incoming, &mut self.internal_messages)
    }

    pub fn close(&mut self) -> Result<(), IOError> {
        match self.state {
            RemoteState::Accepted | RemoteState::Connected => {
                self.state = RemoteState::Closing;
                Ok(())
            },
            RemoteState::Closing | RemoteState::Closed => Err(IOError::new(ErrorKind::NotConnected, ""))
        }
    }


    // Internal ---------------------------------------------------------------
    fn read(&mut self) {

        self.try_connect();

        if self.connection.read(&mut self.incoming).is_err() {
            self.close().ok();
        }

    }

    fn write(&mut self) {

        let messages = self.internal_messages.drain(0..).collect::<Vec<_>>();
        for m in self.timer.receive(messages) {
            self.send_raw(0, m);
        }

        if !self.outgoing.is_empty() && self.connection.write(&self.outgoing[..]).is_ok() {
            self.outgoing.clear();
        }

        self.try_close();

    }

    fn from_connection(connection: C, timer: Timer) -> Self {
        Self {
            connection: connection,
            incoming: Vec::new(),
            outgoing: Vec::new(),
            internal_messages: Vec::new(),
            timer: timer,
            state: RemoteState::Accepted,
            message: PhantomData
        }
    }

    fn send_raw<T: Serialize + DeserializeOwned>(&mut self, prefix: u8, message: T) {
        if let Ok(bytes) = serialize(&message, Infinite) {
            self.outgoing.push(prefix);
            self.outgoing.extend(bytes);
        }
    }

    fn accepted(&self) -> bool {
        self.state == RemoteState::Accepted
    }

    fn connected(&self) -> bool {
        self.state == RemoteState::Connected
    }

    fn closed(&self) -> bool {
        self.state == RemoteState::Closed
    }

    fn try_connect(&mut self)  {
        if self.accepted() {
            self.state = RemoteState::Connected;
        }
    }

    fn try_close(&mut self) {
        if self.state == RemoteState::Closing {
            self.connection.shutdown().ok();
            self.state = RemoteState::Closed;
        }
    }

}

