use ipnet::IpNet;
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::str::FromStr;
use std::time::Duration;
use threadpool::ThreadPool;

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

fn is_port_open(ip: &str, port: u16, timeout: u64) -> bool {
    let ip_addr: Result<IpAddr, _> = ip.parse();
    if let Ok(ip_addr) = ip_addr {
        let socket_addr = SocketAddr::new(ip_addr, port);
        TcpStream::connect_timeout(&socket_addr, Duration::from_millis(timeout)).is_ok()
    } else {
        eprintln!("Could not parse ip: {}", ip);
        std::process::exit(1);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!(
            "Usage: {} <subnet> <port to scan>\n
            \rOptions: 
            \r[-t, --threads]
            \r[-T, --timeout]\n
            \rDefaults: 200 threads, 1000 ms timeout\n",
            args[0]
        );
        std::process::exit(0);
    }

    let cidr = &args[1];
    let port = args[2].parse::<u16>().unwrap();
    let mut num_threads = 200;
    let mut timeout = 1000;

    parse_arg!(&args, "-t", "--threads", num_threads, "-t <threads>");
    parse_arg!(&args, "-T", "--timeout", timeout, "-T <timeout in ms>");

    let pool = ThreadPool::new(num_threads);

    match IpNet::from_str(cidr) {
        Ok(ipnet) => {
            for ip in ipnet.hosts().into_iter() {
                pool.execute(move || {
                    if is_port_open(&ip.to_string(), port, timeout) {
                        println!("{}", ip);
                    }
                });
            }
        }
        Err(e) => {
            eprintln!("Error parsing CIDR: {}", e);
        }
    }

    pool.join();
}
