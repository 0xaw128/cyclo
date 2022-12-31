use std::net::{TcpListener, TcpStream, SocketAddr};
use std::path::Path;
use std::io::{Read, Write};
use std::fs;
use clap::Parser;


#[derive(Parser,Debug)]
#[clap(name="webserver")]
struct Args
{
    /// webserver port
    #[clap(short = 'p', long, value_parser)]
    port: u16,
}

/// Struct representing the barebones for a generic HTTP request
#[derive(Debug)]
struct HttpRequest
{
    method: String,
    uri: String
}

impl HttpRequest
{
    fn new(request_data: String) -> Self
    {
        let req: Vec<&str> = request_data.splitn(2, "\r\n").collect();
        /* status line is GET / HTTP/1.1 etc */
        let status_line = req[0];

        /* this grabs the method like GET */
        let stat: Vec<&str> = status_line.split(" ").collect();
        let method = stat[0].to_string();
        /* this grabs the URI, like / */
        /* TODO: sometimes this panics as stat.len() is 1 so stat[1] is out-of-bounds.
         * not sure why this is happening? */
        let uri = stat[1].to_string();

        HttpRequest { method, uri }
    }
}

/// Handle the HTTP request
fn handle_connection(mut stream: TcpStream)
{
    let mut buf = vec![0;2048];

    stream.read(&mut buf).unwrap();

    let request_data = String::from_utf8_lossy(&buf);
    let request = HttpRequest::new(request_data.to_string());

    let response = if request.method == "GET"
    {

        // parse the URI so if the user navigates to it, it'll just hit a 404
        let filename: String = if request.uri == "/"
        {
            "index.html".to_string()
        }
        else
        {
            request.uri
        };

        let path = format!("./web/{}", filename);

        if Path::new(&path).exists()
        {
            let content = fs::read_to_string(&path).unwrap();

            let mime_type = Path::new(&path).extension().unwrap().to_string_lossy();
            let mime_type = if mime_type == "js"
            {
                "javascript".to_string()
            }
            else
            {
                mime_type.to_string()
            };

            let content_type = format!("text/{}", mime_type);

            // response
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {content_length}\r\nContent-Type: {content_type}\r\n\r\n{body}",
                content_length=content.len(),
                content_type=content_type,
                body=content)
        }
        else
        {
            "HTTP/1.1 404 Not Found\r\n\r\nNot Found".to_string()
        }
    }
    else
    {
        "HTTP/1.1 501 Not Implemented\r\n\r\nNot Implemented".to_string()
    };

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

fn main()
{
    let args = Args::parse();

    // if args.port fails, bind to 3030
    let addrs = [
        SocketAddr::from(([127,0,0,1], args.port)),
        SocketAddr::from(([127,0,0,1], 3030)),
    ];

    let listener = match TcpListener::bind(&addrs[..])
    {
        Ok(listen) => listen,
        Err(error) => panic!("faield to start TcpListener: {:?}", error),
    };

    // use local_addr instead of args.port as an arg of 0 will cause the OS to
    // randomly assign a port
    println!("starting webserver at localhost:{:?}", listener.local_addr().unwrap());

    for stream in listener.incoming()
    {
        let stream = stream.unwrap();

        handle_connection(stream);
    }
}
