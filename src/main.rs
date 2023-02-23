#[macro_use] extern crate rocket;


use rocket::{State, Shutdown};
use rocket::fs::{relative, FileServer};
use rocket::form::Form;
use rocket::response::stream::{EventStream, Event};
use rocket::serde::{Serialize, Deserialize};
use rocket::tokio::sync::broadcast::{channel, Sender, error::RecvError};
use rocket::tokio::select;


// Debug message, clone messages, take form data and transform it to message struct, and serialize / deserialze items 
#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[serde(crate="rocket::serde")]
struct Message {
    #[field(validate = len(..30))] // Room name can only be up to 29 characters long
    pub room: String,
    #[field(validate = len(..20))] // Username can only be up to 19 characters long
    pub username: String,
    pub message: String
}

#[post("/message", data = "<form>")]
// Form data that will be converted to a message and service state (sender that can send messages)
fn post(form: Form<Message>, queue: &State<Sender<Message>>) {
    let _res = queue.send(form.into_inner());
}

/// Returns an infinite stream of server-sent events. Each event is a message
/// pulled from a broadcast queue sent by the `post` handler.
#[get("/events")]
async fn events(queue: &State<Sender<Message>>, mut end: Shutdown) -> EventStream![] {
    // Create a new receiver (listen for messages as they're sent down the channel)
    let mut rx = queue.subscribe();
    EventStream! {
        loop {
            let msg = select! {
                // wait for new messages
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
                // waiting for the shutdown future to resolve (server get's shut down)
                _ = &mut end => break,
            };

            yield Event::json(&msg);
        }
    }
}


#[launch]
fn rocket() -> _{
    rocket::build()
    .manage(channel::<Message>(1024).0)
    .mount("/", routes![post, events])
    .mount("/", FileServer::from(relative!("static")))
}

