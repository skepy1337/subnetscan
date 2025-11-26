use colored::Colorize;
use ipnet::IpNet;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use tokio::sync::Semaphore;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{Duration, timeout},
};

macro_rules! parse_arg {
    ($args:expr, $short:expr, $long:expr, $var:expr, $msg:expr) => {
        if let Some(index) = $args.iter().position(|arg| arg == $short || arg == $long) {
            if let Ok(parsed_num) = $args[index + 1].parse() {
                $var = parsed_num;
            } else {
                eprintln!($msg);
                std::process::exit(1);
            }
        }
    };
}

pub struct Rand {
    state: u8,
}

impl Rand {
    pub fn new() -> Self {
        let seed: u8 = rand::random();
        Self { state: seed }
    }

    fn next(&mut self) -> u8 {
        self.state ^= self.state << 7;
        self.state ^= self.state >> 5;
        self.state ^= self.state << 3;
        self.state
    }
}

async fn is_port_open(ip: IpAddr, port: u16, timeout_ms: Duration) -> bool {
    let socket_addr = SocketAddr::new(ip, port);

    timeout(timeout_ms, TcpStream::connect(&socket_addr))
        .await
        .is_ok()
}

async fn grab_banner(ip: IpAddr, port: u16, timeout_ms: Duration) -> String {
    let addr = SocketAddr::new(ip, port);

    let mut stream = match timeout(timeout_ms, TcpStream::connect(addr)).await {
        Ok(Ok(s)) => s,
        _ => return String::new(),
    };

    let mut rng = Rand::new();
    let mut data: Vec<u8> = Vec::with_capacity(5);
    for _ in 0..data.capacity() {
        data.push(rng.next() / 2 /*128 max*/);
    }

    let string: String = data.iter().map(|&x| x as char).collect();
    let probe = format!("{}\r\n", string);
    let mut response = Vec::new();

    if timeout(timeout_ms, stream.write_all(probe.as_bytes()))
        .await
        .is_err()
    {
        return String::new();
    }

    match timeout(timeout_ms, stream.read_to_end(&mut response)).await {
        Ok(Ok(_)) => String::from_utf8_lossy(&response).to_string(),
        _ => String::new(),
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!(
            "Usage: {} <subnet> <port to scan>\n
            \rOptions: 
            \r[-t, --threads]
            \r[-T, --timeout]
            \r[-n, --nobanner]\n
            \rDefaults: 200 threads, 1000 ms timeout\n",
            args[0]
        );
        std::process::exit(0);
    }

    let cidr = &args[1];
    let port = args[2].parse::<u16>().unwrap();
    let mut num_threads = 200;
    let mut timeout_ms = 1000;
    let mut get_banner = true;

    parse_arg!(&args, "-t", "--threads", num_threads, "-t <threads>");
    parse_arg!(&args, "-T", "--timeout", timeout_ms, "-T <timeout in ms>");

    for arg in &args {
        if arg.contains("-n") || arg.contains("--nobanner") {
            get_banner = false;
        }
    }

    // limit the number of async tasks
    let sem = std::sync::Arc::new(Semaphore::new(num_threads));

    match IpNet::from_str(cidr) {
        Ok(ipnet) => {
            println!("Scanning {} IPs\n", ipnet.hosts().count());

            for ip in ipnet.hosts().into_iter() {
                let permit = sem.clone().acquire_owned().await.unwrap();
                tokio::spawn(async move {
                    if !is_port_open(ip, port, Duration::from_millis(timeout_ms)).await {
                        return;
                    }

                    if !get_banner {
                        println!("{}", ip.to_string().bright_green());
                    } else {
                        let banner = grab_banner(ip, port, Duration::from_millis(timeout_ms)).await;
                        if !banner.is_empty() {
                            println!(
                                "{}:\n\n\r{}\n",
                                ip.to_string().bright_green(),
                                banner.trim_end()
                            );
                        } else {
                            println!("{}", ip.to_string().bright_green());
                        }
                    }
                    drop(permit);
                });
            }
        }
        Err(e) => {
            eprintln!(
                "Error parsing CIDR: {}
            \rExample: 192.168.1.0/24\n",
                e
            );
        }
    }
}
