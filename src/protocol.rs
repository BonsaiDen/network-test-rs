// Copyright (c) 2017 Ivo Wetzel

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


// STD Dependencies -----------------------------------------------------------
use std::time::Duration;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::io::{Error as IOError, ErrorKind};
use std::net::{SocketAddr, Shutdown, ToSocketAddrs};


// Connection Abstraction -----------------------------------------------------
pub trait Protocol {
    type Host: Host<Connection = Self::Connection>;
    type Connection: Connection;
}

pub trait Host {
    type Connection: Connection;
    fn bind<A: ToSocketAddrs>(addr: A) -> Result<Self, IOError> where Self: Sized;
    fn accept(&mut self) -> Result<Self::Connection, IOError> where Self: Sized;
    fn shutdown(self) -> Result<(), IOError> where Self: Sized;
}

pub trait Connection {
    fn connect<A: ToSocketAddrs>(addr: A, timeout: Duration) -> Result<Self, IOError> where Self: Sized;
    fn peer_addr(&self) -> Result<SocketAddr, IOError> where Self: Sized;
    fn read(&mut self, &mut Vec<u8>) -> Result<usize, IOError> where Self: Sized;
    fn write(&mut self, &[u8]) -> Result<usize, IOError> where Self: Sized;
    fn shutdown(&mut self) -> Result<(), IOError> where Self: Sized;
}


// TPC Protocol ---------------------------------------------------------------
pub struct TCP;
impl Protocol for TCP {
    type Host = TcpHost;
    type Connection = TcpConnection;
}


pub struct TcpHost {
    listener: TcpListener
}

impl Host for TcpHost {

    type Connection = TcpConnection;

    fn bind<A: ToSocketAddrs>(addr: A) -> Result<Self, IOError> where Self: Sized {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener: listener
        })
    }

    fn accept(&mut self) -> Result<TcpConnection, IOError> where Self: Sized {
        let (stream, addr) = self.listener.accept()?;
        stream.set_nodelay(true)?;
        stream.set_nonblocking(true)?;
        Ok(TcpConnection {
            stream: stream,
            peer_addr: Some(addr)
        })
    }

    fn shutdown(self) -> Result<(), IOError> where Self: Sized {
        Ok(())
    }

}

pub struct TcpConnection {
    stream: TcpStream,
    peer_addr: Option<SocketAddr>
}

impl Connection for TcpConnection {

    fn connect<A: ToSocketAddrs>(addr: A, timeout: Duration) -> Result<Self, IOError> where Self: Sized {
        if let Some(addr) = addr.to_socket_addrs()?.next() {
            let stream = TcpStream::connect_timeout(&addr, timeout)?;
            stream.set_nodelay(true)?;
            stream.set_nonblocking(true)?;
            Ok(Self {
                stream: stream,
                peer_addr: Some(addr)
            })

        } else {
            Err(IOError::new(ErrorKind::AddrNotAvailable, ""))
        }
    }

    fn peer_addr(&self) -> Result<SocketAddr, IOError> where Self: Sized{
        if let Some(addr) = self.peer_addr {
            Ok(addr)

        } else {
            Err(IOError::new(ErrorKind::NotConnected, ""))
        }
    }

    fn read(&mut self, buffer: &mut Vec<u8>) -> Result<usize, IOError> where Self: Sized {
        if let Ok(bytes) = self.stream.read_to_end(buffer) {
            if bytes == 0 {
                self.stream.shutdown(Shutdown::Both).ok();
                Err(IOError::new(ErrorKind::ConnectionReset, ""))

            } else {
                Ok(bytes)
            }

        } else {
            Ok(0)
        }
    }

    fn write(&mut self, bytes: &[u8]) -> Result<usize, IOError> where Self: Sized {
        self.stream.write_all(&bytes[..])?;
        self.stream.flush()?;
        Ok(bytes.len())
    }

    fn shutdown(&mut self) -> Result<(), IOError> where Self: Sized {
        self.stream.shutdown(Shutdown::Both)
    }

}

