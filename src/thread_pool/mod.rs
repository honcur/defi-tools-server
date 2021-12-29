use std::thread;
use std::thread::JoinHandle;
use std::sync::{ mpsc, mpsc::Receiver, mpsc::Sender, Arc, Mutex };
enum Message {
    Stop,
    JobMessage(Job)
}

struct Work {
    id : usize,
    join: Option<JoinHandle<()>>
}

pub struct ThreadPool {
    works: Vec<Work>,
    sender: Sender<Message>
}


type Job = Box<dyn FnOnce() + Send + 'static>;
impl Work { 
    pub fn new(id: usize, receiver:Arc<Mutex<Receiver<Message>>>) -> Work {
        let join = thread::spawn(move || {
            loop {
               let message = receiver.lock().unwrap().recv().unwrap();
               match message {
                   Message::JobMessage(job) => {
                       job();
                   },
                   Message::Stop => break
               }
            }
        });
        Work {
            id,
            join: Some(join)
        }
    }
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);
        let mut works = Vec::with_capacity(size);
        let (tx, rx): (Sender<Message>, Receiver<Message>) = mpsc::channel();
        let arc = Arc::new(Mutex::new(rx));
        for id in 0 .. size {
            works.push(Work::new(id, Arc::clone(&arc)));
        }
        ThreadPool {
            works,
            sender: tx
        }
    }

    pub fn execute<F>(&self, f: F)  
        where F: FnOnce() + Send + 'static{
        self.sender.send(Message::JobMessage(Box::new(f))).unwrap();
    }
}

impl Drop for ThreadPool {

    fn drop(&mut self) {
        for _ in self.works.iter() {
            self.sender.send(Message::Stop).unwrap();
        }

        for work in self.works.iter_mut() {
           if let Some(join) = work.join.take() {
               join.join().unwrap();
           }
        }
    }
}