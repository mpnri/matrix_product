#![allow(unused)]

use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::serde::{json::Json, Deserialize};
use rocket::{get, launch, options, post, routes, Request, Response, State};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Cross-Origin-Resource-Sharing Fairing",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

#[get("/")]
fn index() -> &'static str {
    println!("get /");
    "Hello, world!"
}

type Matrix = Vec<Vec<i32>>;

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct Data {
    matrix_a: Matrix,
    matrix_b: Matrix,
}

const MAX_WORKERS: isize = 4; // maximum number of worker processes???
const TASK_SIZE: usize = 2; // maximum number of rows or columns???

#[post("/", format = "json", data = "<data>")]
fn matrix_handler(
    data: Json<Data>,
    rx: &State<Arc<Mutex<Receiver<usize>>>>,
    tx: &State<Arc<Mutex<Sender<usize>>>>,
) -> String {
    println!("Received matrix: {:?}", data.matrix_a);
    println!("Received matrix: {:?}", data.matrix_b);

    let height = data.matrix_a.len();
    let width = data.matrix_b[0].len();
    let result = Arc::new(Mutex::new(vec![vec![0; width]; height]));

    let semaphore = Arc::new(Semaphore::new(MAX_WORKERS));
    let mut semaphore_number = MAX_WORKERS;

    let rx = Arc::clone(&rx);
    let sm = Arc::clone(&semaphore);
    thread::spawn(move || loop {
        let rx = rx.lock().unwrap();
        let cnt = rx.recv().unwrap();
        println!("recv: {}", cnt);
        if cnt == 0 {
            println!("end");
            break;
        } else if cnt > 0 {
            let cnt = isize::try_from(cnt).expect("err usize to isize");
            let change = cnt - semaphore_number;
            sm.change_count(change);
            semaphore_number = cnt;
        }
    });

    //let w =thread::spawn(move || {
    let res = handle_request(
        Arc::new(data.matrix_a.clone()),
        Arc::new(data.matrix_b.clone()),
        result.clone(),
        semaphore,
    );

    //* End Checking for worker count */
    let tx = tx.lock().unwrap();
    tx.send(0).unwrap();
    println!("res: {}", res);
    res
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct WorkerData {
    count: usize,
}

#[post("/workers", format = "json", data = "<data>")]
fn workers_number_handler(data: Json<WorkerData>, tx: &State<Arc<Mutex<Sender<usize>>>>) -> String {
    let tx = tx.lock().unwrap();
    println!("workers_number_handler: {}", data.count);
    tx.send(data.count);
    "OK".to_string()
}

#[options("/")]
fn options() -> String {
    // Check if the GET and POST methods are supported
    //let allowed_methods = vec!["GET", "POST"];
    "OK".to_string()
}

#[launch]
fn rocket() -> _ {
    let (tx, rx): (Sender<usize>, Receiver<usize>) = channel();
    let tx = Arc::new(Mutex::new(tx));
    let rx = Arc::new(Mutex::new(rx));

    rocket::build().attach(CORS).manage(rx).manage(tx).mount(
        "/",
        routes![matrix_handler, workers_number_handler, options],
    )
}

fn handle_request(
    matrix_a: Arc<Matrix>,
    matrix_b: Arc<Matrix>,
    result: Arc<Mutex<Matrix>>,
    semaphore: Arc<Semaphore>,
) -> String {
    let mut workers = Vec::new();
    let mut i = 0;
    let mut j = 0;
    //* thread::sleep(Duration::from_secs(3));
    for row in 0..matrix_a.len() {
        i += 1;
        j = 0;
        for col in 0..matrix_b[0].len() {
            j += 1;
            let semaphore = Arc::clone(&semaphore);
            println!("bef {}, {}", i, j);
            semaphore.wait();
            println!("aft {}, {}", i, j);

            let result = Arc::clone(&result);
            let matrix_a = Arc::clone(&matrix_a);
            let matrix_b = Arc::clone(&matrix_b);

            let worker = thread::spawn(move || {
                let mut sum = 0;
                for i in 0..matrix_b.len() {
                    sum += matrix_a[row][i] * matrix_b[i][col];
                }
                let mut result = result.lock().unwrap();
                result[row][col] = sum;
                //* thread::sleep(Duration::from_secs(3));
                semaphore.signal();
            });
            workers.push(worker);
            // workers.remove(index)
        }
    }
    println!("workers length: {}", workers.len());
    for worker in workers {
        worker.join().unwrap();
    }

    let response = format!(
        "{}\r\n{}\r\n{}\n{}\n\n{}\n{}\n\n{}\n{}\r\n\r\n",
        "HTTP/1.1 200 OK",
        "Content-Type: text/plain",
        "matrix_a:",
        matrix_to_string(&matrix_a),
        "matrix_b:",
        matrix_to_string(&matrix_b),
        "result:",
        matrix_to_string(&*result.lock().unwrap())
    );
    response
}

fn matrix_to_string(matrix: &Matrix) -> String {
    let mut result = String::new();
    let mut max_val_len = 0;
    matrix.iter().for_each(|row| {
        row.iter().for_each(|val| {
            max_val_len = std::cmp::max(max_val_len, val.to_string().len());
        })
    });

    for row in matrix {
        result.push_str(
            &row.iter()
                .map(|x| x.to_string())
                .map(|x| {
                    if x.len() == max_val_len {
                        x
                    } else {
                        "0".repeat(max_val_len - x.len()) + &x
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
        );
        result.push_str("\n");
    }

    result
}

struct Semaphore {
    count: Mutex<isize>,
    condvar: Condvar,
}

impl Semaphore {
    pub fn new(count: isize) -> Semaphore {
        Semaphore {
            count: Mutex::new(count),
            condvar: Condvar::new(),
        }
    }

    pub fn wait(&self) {
        let mut count = self.count.lock().unwrap();
        while *count == 0 {
            count = self.condvar.wait(count).unwrap();
        }
        *count -= 1;
        println!("wait {}", count);
    }

    pub fn signal(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        println!("signal {}", count);
        self.condvar.notify_one();
    }

    pub fn change_count(&self, num: isize) {
        let mut count = self.count.lock().unwrap();
        *count += num;
    }
}
