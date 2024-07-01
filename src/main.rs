use std::{
    env,
    fs::File,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::Path,
};

use flate2::{bufread::GzEncoder, Compression};

fn main() {
    let addr = SocketAddr::from(([0, 0, 0, 0], 4221));
    let listener = TcpListener::bind(addr).unwrap();
    println!("Listening from {}", addr);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handler(stream),
            Err(e) => {
                eprintln!("error: {}", e);
            }
        }
    }
}

fn handler(mut stream: TcpStream) {
    // request
    let mut req_buf: Vec<u8> = vec![0; 512];
    let mut res: Response = Response::create(HttpStatus::Continue, None, None);
    match stream.read(&mut req_buf) {
        Ok(len) => {
            let req = Request::create(String::from_utf8_lossy(&req_buf[..len]).to_string());
            // check version | only handling http/1.1
            if let HttpVersion::HTTP1_1 = req.req_line.version {
                // handle encoding
                if let Some(header) = req.has_header("Accept-Encoding") {
                    res.set_encoding(HttpEncoding::first_valid(header))
                }
                // Choose response
                let target = req.req_line.target.as_str();
                match target {
                    "/" => res.set_status(HttpStatus::OK),
                    "/user-agent" => user_agent(&req, &mut res),
                    _ if target.starts_with("/echo") => echo(target, &mut res),
                    _ if target.starts_with("/files") => files(&req, &mut res),
                    _ => res.set_status(HttpStatus::NotFound),
                }
            } else {
                res.set_status(HttpStatus::HTTPVersionNotSupported)
            }
        }
        Err(e) => eprintln!("Request error:{}", e),
    }

    // response
    match stream.write_all(&res.value()) {
        Ok(_) => {
            // dbg!(res);
            println!(
                "Response: {} -> {}",
                res.status.value(),
                stream.peer_addr().unwrap()
            )
        }
        Err(e) => eprintln!("Response error:{}", e),
    }
}

// Routing?
fn echo(target: &str, res: &mut Response) {
    if target.len() < 7 {
        // lenght of /echo/
        res.set_status(HttpStatus::BadRequest);
        return;
    }
    let content = &target[6..]; // from /echo/ (7th position -> 6)
    res.set_status(HttpStatus::OK);
    res.body = Some(HttpBody::create(content.to_string()));
}

fn user_agent(req: &Request, res: &mut Response) {
    let headers = <Option<Vec<Header>> as Clone>::clone(&req.headers).unwrap();
    // dbg!(headers);
    if let Some(header) = headers.iter().find(|header| header.name == "User-Agent") {
        res.set_status(HttpStatus::OK);
        let content = &header.value;
        res.body = Some(HttpBody::create(content.to_string()));
        return;
    }
    res.set_status(HttpStatus::BadRequest);
}

fn files(req: &Request, res: &mut Response) {
    let target = &req.req_line.target;
    if target.len() < 8 {
        // length of /files/
        res.set_status(HttpStatus::BadRequest);
        return;
    }
    let query = &target[7..]; // /files/..
    let dir = get_config("directory").unwrap_or("/tmp/rust-server".to_string());

    let file_path = format!("{}/{}", dir, query);
    let path = Path::new(&file_path);

    match req.req_line.method {
        HttpMethod::Get => {
            if !path.exists() {
                res.set_status(HttpStatus::NotFound);
                return;
            }
            match File::open(&file_path) {
                Ok(mut file) => {
                    let mut content = String::new();
                    match file.read_to_string(&mut content) {
                        Ok(size) => {
                            let content_type =
                                Header::create("Content-type", "application/octet-stream");
                            let content_length =
                                Header::create("Content-length", &size.to_string());
                            res.headers = Some(vec![content_type, content_length]);

                            res.set_status(HttpStatus::OK);
                            res.body = Some(HttpBody::create(content.to_string()));
                        }
                        Err(err) => {
                            res.set_status(HttpStatus::InternalServerError);
                            res.body = Some(HttpBody::create(err.to_string()));
                        }
                    }
                }
                Err(_) => {
                    res.set_status(HttpStatus::InternalServerError);
                }
            }
        }
        HttpMethod::Post => {
            if path.exists() {
                res.set_status(HttpStatus::MethodNotAllowed);
                return;
            }
            match File::create(&file_path) {
                Ok(mut file) => {
                    let content = req.body.as_ref().unwrap().value();
                    match file.write_all(&content) {
                        Ok(_) => {
                            res.set_status(HttpStatus::Created);
                        }
                        Err(_) => {
                            res.set_status(HttpStatus::InternalServerError);
                        }
                    }
                }
                Err(_) => {
                    res.set_status(HttpStatus::InternalServerError);
                }
            }
        }
        _ => res.set_status(HttpStatus::MethodNotAllowed),
    }
}

// Utils
fn get_config(config: &str) -> Option<String> {
    // TODO: get from config file
    get_arg(config)
}

fn get_arg(config: &str) -> Option<String> {
    let args: Vec<String> = env::args().collect();
    let config_flag = format!("--{}", config);
    args.windows(2).find_map(|window| {
        if window[0] == config_flag {
            Some(window[1].clone())
        } else {
            None
        }
    })
}

// STRUCTS...
#[derive(Debug)]
struct Request {
    req_line: RequestLine,
    headers: Option<Vec<Header>>,
    body: Option<HttpBody>,
}

impl Request {
    fn create(req: String) -> Self {
        let mut req_split = req.split("\r\n");
        let req_line = RequestLine::create(req_split.next().unwrap());
        // Headers
        let mut headers: Vec<Header> = Vec::new();
        for line in req_split.by_ref() {
            if line.is_empty() {
                // if empty line double CRLF -> Body
                break;
            }
            // read header
            let (name, value) = line.split_once(": ").unwrap();
            // println!("Request: Header({name}) = {value}");
            headers.push(Header::create(name, value))
        }
        // check if empty headers
        let headers = if headers.is_empty() {
            None
        } else {
            Some(headers)
        };

        // Body
        let mut body: String = Default::default();
        for line in req_split {
            body.push_str(line);
        }
        // check if empty headers
        let body = if body.is_empty() {
            None
        } else {
            Some(HttpBody::create(body))
        };

        // dbg!(&req_line);
        // dbg!(&headers);
        // dbg!(&body);

        Request {
            req_line,
            headers,
            body,
        }
    }
    fn has_header(&self, query: &str) -> Option<&Header> {
        if let Some(headers) = &self.headers {
            return headers.iter().find(|header| header.name == query);
        }
        None
    }
}

#[derive(Debug)]
struct RequestLine {
    method: HttpMethod,
    target: String,
    version: HttpVersion,
}

impl RequestLine {
    fn create(req_line: &str) -> Self {
        let mut i = req_line.split(' ');
        RequestLine {
            method: HttpMethod::from_str(i.next().unwrap()),
            target: i.next().unwrap().to_string(),
            version: HttpVersion::from_str(i.next().unwrap()),
        }
    }
}

#[derive(Debug)]
enum HttpMethod {
    // [MDN](https://developer.mozilla.org/es/docs/Web/HTTP/Methods)
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
    Patch,
}

impl HttpMethod {
    fn from_str(method: &str) -> Self {
        match method {
            "GET" => Self::Get,
            "HEAD" => Self::Head,
            "POST" => Self::Post,
            "PUT" => Self::Put,
            "DELETE" => Self::Delete,
            "CONNECT" => Self::Connect,
            "OPTIONS" => Self::Options,
            "TRACE" => Self::Trace,
            "PATCH" => Self::Patch,
            _ => Self::Get,
        }
    }
}

#[derive(Debug)]
enum HttpVersion {
    HTTP1_1,
    HTTP2,
    HTTP3,
}

impl HttpVersion {
    fn value(&self) -> &str {
        match *self {
            Self::HTTP1_1 => "HTTP/1.1",
            Self::HTTP2 => "HTTP/2",
            Self::HTTP3 => "HTTP/3",
        }
    }

    fn from_str(version: &str) -> Self {
        match version {
            "HTTP/1.1" => Self::HTTP1_1,
            "HTTP/2" => Self::HTTP2,
            "HTTP/3" => Self::HTTP3,
            _ => Self::HTTP1_1,
        }
    }
}

#[derive(Debug)]
/* from [RFC 2616](https://www.rfc-editor.org/rfc/rfc2616) */
enum HttpStatus {
    Continue,
    // SwitchingProtocols,
    OK,
    Created,
    // Accepted,
    // NonAuthoritativeInformation,
    // NoContent,
    // ResetContent,
    // PartialContent,
    // MultipleChoices,
    // MovedPermanently,
    // Found,
    // SeeOther,
    // NotModified,
    // UseProxy,
    // TemporaryRedirect,
    BadRequest,
    // Unauthorized,
    // PaymentRequired,
    // Forbidden,
    NotFound,
    MethodNotAllowed,
    // NotAcceptable,
    // ProxyAuthenticationRequired,
    // RequestTimeout,
    // Conflict,
    // Gone,
    // LengthRequired,
    // PreconditionFailed,
    // RequestEntityTooLarge,
    // RequestURITooLarge,
    // UnsupportedMediaType,
    // Requestedrangenotsatisfiable,
    // ExpectationFailed,
    InternalServerError,
    // NotImplemented,
    // BadGateway,
    // ServiceUnavailable,
    // GatewayTimeout,
    HTTPVersionNotSupported,
}

impl HttpStatus {
    fn value(&self) -> (u16, &str) {
        match *self {
            Self::Continue => (100, "Continue"),
            // Self::SwitchingProtocols => (101, "Switching Protocols"),
            Self::OK => (200, "OK"),
            Self::Created => (201, "Created"),
            // Self::Accepted => (202, "Accepted"),
            // Self::NonAuthoritativeInformation => (203, "Non-Authoritative Information"),
            // Self::NoContent => (204, "No Content"),
            // Self::ResetContent => (205, "Reset Content"),
            // Self::PartialContent => (206, "Partial Content"),
            // Self::MultipleChoices => (300, "Multiple Choices"),
            // Self::MovedPermanently => (301, "Moved Permanently"),
            // Self::Found => (302, "Found"),
            // Self::SeeOther => (303, "See Other"),
            // Self::NotModified => (304, "Not Modified"),
            // Self::UseProxy => (305, "Use Proxy"),
            // Self::TemporaryRedirect => (307, "Temporary Redirect"),
            Self::BadRequest => (400, "Bad Request"),
            // Self::Unauthorized => (401, "Unauthorized"),
            // Self::PaymentRequired => (402, "Payment Required"),
            // Self::Forbidden => (403, "Forbidden"),
            Self::NotFound => (404, "Not Found"),
            Self::MethodNotAllowed => (405, "Method Not Allowed"),
            // Self::NotAcceptable => (406, "Not Acceptable"),
            // Self::ProxyAuthenticationRequired => (407, "Proxy Authentication Required"),
            // Self::RequestTimeout => (408, "Request Time-out"),
            // Self::Conflict => (409, "Conflict"),
            // Self::Gone => (410, "Gone"),
            // Self::LengthRequired => (411, "Length Required"),
            // Self::PreconditionFailed => (412, "Precondition Failed"),
            // Self::RequestEntityTooLarge => (413, "Request Entity Too Large"),
            // Self::RequestURITooLarge => (414, "Request-URI Too Large"),
            // Self::UnsupportedMediaType => (415, "Unsupported Media Type"),
            // Self::Requestedrangenotsatisfiable => (416, "Requested range not satisfiable"),
            // Self::ExpectationFailed => (417, "Expectation Failed"),
            Self::InternalServerError => (500, "Internal Server Error"),
            // Self::NotImplemented => (501, "Not Implemented"),
            // Self::BadGateway => (502, "Bad Gateway"),
            // Self::ServiceUnavailable => (503, "Service Unavailable"),
            // Self::GatewayTimeout => (504, "Gateway Time-out"),
            Self::HTTPVersionNotSupported => (505, "HTTP Version not supported"),
        }
    }
}

#[derive(Debug)]
enum HttpEncoding {
    Gzip,
    Compress,
    Deflate,
    Identity,
    Br,
}

impl HttpEncoding {
    fn value(&self) -> &str {
        match *self {
            Self::Gzip => "gzip",
            Self::Compress => "compress",
            Self::Deflate => "deflate",
            Self::Identity => "identity",
            Self::Br => "br",
        }
    }

    fn is_valid(&self) -> bool {
        match *self {
            Self::Gzip => true,
            _ => false,
        }
    }

    fn from_str(version: &str) -> Option<Self> {
        match version {
            "gzip" => Some(Self::Gzip),
            "compress" => Some(Self::Compress),
            "deflate" => Some(Self::Deflate),
            "identity" => Some(Self::Identity),
            "br" => Some(Self::Br),
            _ => None,
        }
    }

    fn first_valid(header: &Header) -> Option<Self> {
        let vals = header.value.clone();
        vals.split(", ").find_map(|method| {
            let encoding = match HttpEncoding::from_str(method) {
                Some(v) => v,
                None => return None,
            };
            if encoding.is_valid() {
                Some(encoding)
            } else {
                None
            }
        })
    }
}

#[derive(Debug)]
struct StatusLine {
    status: HttpStatus,
    version: HttpVersion,
}

impl StatusLine {
    fn create(status: HttpStatus, version: Option<HttpVersion>) -> StatusLine {
        StatusLine {
            status,
            version: version.unwrap_or(HttpVersion::HTTP1_1),
        }
    }

    fn value(&self) -> String {
        let (code, msg) = self.status.value();
        let ver = self.version.value();
        format!("{ver} {code} {msg}")
    }
}

#[derive(Debug, Clone)]
struct Header {
    name: String,
    value: String,
}

impl Header {
    fn create(name: &str, value: &str) -> Self {
        Header {
            name: name.to_string(),
            value: value.to_string(),
        }
    }
    fn value(&self) -> String {
        format!("{}: {}\r\n", self.name, self.value)
    }
}

#[derive(Debug)]
struct HttpBody {
    content: Vec<u8>,
    content_compresed: Option<Vec<u8>>,
}

impl HttpBody {
    fn create(body: String) -> Self {
        let content = body.as_bytes().to_vec();
        HttpBody {
            content,
            content_compresed: None,
        }
    }
    fn value(&self) -> Vec<u8> {
        if let Some(content) = &self.content_compresed {
            return content.clone();
        }
        self.content.clone()
    }
    fn len(&self) -> usize {
        match &self.content_compresed {
            Some(content) => content.len(),
            None => self.content.len(),
        }
    }
    fn encode(&mut self, method: &HttpEncoding) {
        self.content_compresed = match method {
            HttpEncoding::Gzip => Some(gzip(&self.content)),
            _ => None,
        };
    }
}

#[derive(Debug)]
struct Response {
    status: StatusLine,
    headers: Option<Vec<Header>>,
    body: Option<HttpBody>,
    encoding: Option<HttpEncoding>,
}

impl Response {
    fn create(status: HttpStatus, headers: Option<Vec<Header>>, body: Option<HttpBody>) -> Self {
        Response {
            status: StatusLine::create(status, None),
            headers,
            body,
            encoding: None,
        }
    }

    fn value(&mut self) -> Vec<u8> {
        let mut res: Vec<u8> = Vec::new();

        if let Some(encoding) = &self.encoding {
            if let Some(body) = &mut self.body {
                body.encode(encoding);
            }
        }
        let status: String = self.status.value();
        let headers: String = match &self.headers {
            Some(headers) => {
                let mut s: String = Default::default();
                for header in headers {
                    s.push_str(header.value().as_str());
                }
                if let Some(encoding) = &self.encoding {
                    s.push_str(
                        Header::create("Content-Encoding", encoding.value())
                            .value()
                            .as_str(),
                    );
                }
                s
            }
            None => {
                if let Some(body) = &self.body {
                    let content_type = Header::create("Content-type", "text/plain");
                    let content_length = Header::create("Content-length", &body.len().to_string());

                    let mut headers = vec![content_type, content_length];

                    if let Some(encoding) = &self.encoding {
                        headers.push(Header::create("Content-Encoding", encoding.value()));
                    }

                    let mut s: String = Default::default();
                    for header in headers {
                        s.push_str(header.value().as_str());
                    }
                    s
                } else {
                    "".to_string()
                }
            }
        };
        res.extend(
            format!("{status}\r\n{headers}\r\n")
                .to_string()
                .into_bytes(),
        );
        let body: Vec<u8> = match &self.body {
            Some(body) => body.value(),
            None => "".into(),
        };
        res.extend(body);
        res
    }

    fn set_status(&mut self, status: HttpStatus) {
        self.status.status = status;
    }

    fn set_encoding(&mut self, encoding: Option<HttpEncoding>) {
        self.encoding = encoding;
    }
}

// Encodings...

fn gzip(body: &Vec<u8>) -> Vec<u8> {
    let mut buff = Vec::new();
    let mut encoder = GzEncoder::new(&body[..], Compression::default());
    _ = encoder.read_to_end(&mut buff);
    buff
}
