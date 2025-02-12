use colored::Colorize;
use ipnet::IpNet;
use rand::Rng;
use std::io::{Read, Write};
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
fn is_port_open(ip: IpAddr, port: u16, timeout: u64) -> bool {
    let socket_addr = SocketAddr::new(ip, port);
    TcpStream::connect_timeout(&socket_addr, Duration::from_millis(timeout)).is_ok()
}

fn grab_banner(ip: IpAddr, port: u16, timeout: u64) -> String {
    let socket_addr = SocketAddr::new(ip, port);

    if let Ok(mut stream) = TcpStream::connect_timeout(&socket_addr, Duration::from_millis(timeout))
    {
        let mut data: [u8; 3] = [0; 3];
        let mut rng = rand::rng();
        rng.fill(&mut data);
        let probe = format!("{}\r\n", String::from_utf8_lossy(&data));

        let mut response = Vec::new();
        let _ = stream.set_read_timeout(Some(Duration::from_millis(timeout)));
        let _ = stream.write_all(probe.as_bytes());
        let _ = stream.read_to_end(&mut response);

        return String::from_utf8_lossy(&response).to_string();
    }

    String::default()
}

fn main() {
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
    let mut timeout = 1000;
    let mut get_banner = true;

    parse_arg!(&args, "-t", "--threads", num_threads, "-t <threads>");
    parse_arg!(&args, "-T", "--timeout", timeout, "-T <timeout in ms>");

    for arg in &args {
        if arg.contains("-n") || arg.contains("--nobanner") {
            get_banner = false;
        }
    }

    let pool = ThreadPool::new(num_threads);

    match IpNet::from_str(cidr) {
        Ok(ipnet) => {
            println!("Scanning {} IPs\n", ipnet.hosts().count());

            for ip in ipnet.hosts().into_iter() {
                pool.execute(move || {
                    if !is_port_open(ip, port, timeout) {
                        return;
                    }

                    if !get_banner {
                        println!("{}", ip.to_string().bright_green());
                    } else {
                        let banner = grab_banner(ip, port, timeout);
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

    pool.join();
}
