#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use rocket::State;
//use std::io::{self, Read};
//use rocket::{Request, Data, Outcome::*};
//use rocket::data::{FromData, Outcome, Transform, Transformed};
//use rocket::http::Status;
use rocket_contrib::json::Json;
use serde::Deserialize;
use std::process::Command;
use std::thread;
//use std::sync::mpsc::channel;
use std::sync::Mutex;
use std::sync::Arc;
use std::collections::VecDeque;
use std::str;

#[derive(Clone)]
struct WorkQueue<T: Send> {
    inner: Arc<Mutex<VecDeque<T>>>,
}

impl<T: Send + Copy> WorkQueue<T> {

    fn new() -> Self { 
        Self { inner: Arc::new(Mutex::new(VecDeque::new())) } 
    }

    fn get_work(&self) -> Option<T> {
        if let Ok(mut q) = self.inner.lock() {
            q.pop_front()
        } else {
            panic!("WorkQueue::get_work() tried to lock a poisoned mutex")
        }
    }

    fn add_work(&self, work: T) -> usize {
        if let Ok(mut q) = self.inner.lock() {
            q.push_back(work);
            q.len()
        } else {
            panic!("WorkQueue::get_work() tried to lock a poisoned mutex")
        }
    }

}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct Video {
    link: String,
}

impl Video {

    pub fn download(self) -> bool {
        let output = Command::new("youtube-dl")
                .args(&["-f", "best", "--extract-audio", "--audio-format", "mp3", &self.link])
                .output()
                .expect("failed to execute process");
        let output = match str::from_utf8(&output.stdout) {
            Ok(res) => res.into(),
            Err(e) => format!("Youtube-dl Error: {:?}", e)
        };
        println!("Youtube-dl: {}", output);
        false
    }

}

/*const NAME_LIMIT: u64 = 256;

enum NameError {
    Io(io::Error),
    Parse
}

impl<'a> FromData<'a> for Video<'a> {
    type Error = NameError;
    type Owned = String;
    type Borrowed = str;

    fn transform(_: &Request, data: Data) -> Transform<Outcome<Self::Owned, Self::Error>> {
        let mut stream = data.open().take(NAME_LIMIT);
        let mut string = String::with_capacity((NAME_LIMIT / 2) as usize);
        let outcome = match stream.read_to_string(&mut string) {
            Ok(_) => Success(string),
            Err(e) => Failure((Status::InternalServerError, NameError::Io(e)))
        };
        Transform::Borrowed(outcome)
    }

    fn from_data(_: &Request, outcome: Transformed<'a, Self>) -> Outcome<Self, Self::Error> {
        let string = outcome.borrowed()?;
        let splits: Vec<&str> = string.split(" ").collect();
        if splits.len() != 2 || splits.iter().any(|s| s.is_empty()) {
            return Failure((Status::UnprocessableEntity, NameError::Parse));
        }
        Success(Video { link: "" })
    }
}*/

#[post("/addVideo", format = "application/json", data = "<video>")]
fn add_video(queue: State<WorkQueue<Video>>, video: Json<Video>) -> String {
    let v: Video = video.into_inner();
    queue.inner().add_work(v);
    "LOL".into()
}


fn main() {
    let queue: WorkQueue<Video> = WorkQueue::new();
    //let (res_tx, res_rx) = channel();
    let mut threads = Vec::new();
    for _ in 0..5 {
        let tq = queue.clone();
        //let t_res_tx = res_tx.clone();
        let handle = thread::spawn(move || {
            if let Some(work) = tq.get_work() {
                work.download();
            }
            std::thread::yield_now();
        });
        threads.push(handle);
    }
    rocket::ignite().manage(queue).mount("/", routes![add_video]).launch();
}


