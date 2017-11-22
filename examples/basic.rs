// Crates ---------------------------------------------------------------------
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate bincode;
extern crate cobalt_two;


// STD Dependencies -----------------------------------------------------------
use std::thread;
use std::io::Error as IOError;
use std::time::Duration;


// External Dependencies ------------------------------------------------------
use cobalt_two::{Client, Server, TCP};


#[derive(Debug, Serialize, Deserialize)]
enum Message {
    Hello,
    Text
}

fn client() -> Result<(), IOError> {

    // TODO expose tick rate set by server via client
    // TODO bytes send / received per second for client

    // client.ticks_per_second()
    // client.ticks()
    // client.time()

    // client.bytes_sent();
    // client.bytes_received();
    let mut client = Client::<TCP, Message>::new(30);
    client.connect("127.0.0.1:7564", Duration::from_millis(500))?;

    let mut ticks = 0;
    loop {

        println!("[Client] RTT: {} Clock: {} (behind)", client.rtt().round(), client.clock().round());
        for m in client.receive()? {
            println!("[Client] [Remote] [Message] {:?}", m);
        }

        if ticks % 3 == 0 {
            client.send(Message::Text)?;
        }
        ticks += 1;

        client.sleep();

    }

    Ok(())

}

fn run_server() -> Result<(), IOError> {

    #[derive(Default)]
    struct Data;

    // TODO configure tick rate only on server and transmit to client behind the scenes
    // TODO bytes send / received per second for remotes and server total

    // server.ticks_per_second()
    // server.ticks()
    // server.time()

    // server.bytes_sent();
    // server.bytes_received();

    // r.bytes_sent();
    // r.bytes_received();

    let mut server = Server::<TCP, Message, Data>::new(30);
    server.bind("0.0.0.0:7564")?;


    loop {

        for &mut (ref mut r, _) in server.accepted_with(|_| Some(Data::default())) {
            println!("[Server] [Remote] Accepted");
            r.send(Message::Hello);
        }

        for &mut (ref mut r, _) in server.connected() {
            //println!("[Server] [Remote] Connected {:?} ({}ms rtt)", r.peer_addr(), r.rtt());
            for m in r.receive() {
                //println!("[Server] [Remote] [Message] {:?}", m);
            }
        }

        for (r, _) in server.closed() {
            println!("[Server] [Remote] Closed");
        }

        server.sleep();

    }

    server.shutdown();

}

fn main() {

    let server_handle = thread::spawn(|| {
        run_server()
    });

    let client_handle = thread::spawn(|| {
        client().ok();
    });

    client_handle.join().ok();
    server_handle.join().ok();

}

