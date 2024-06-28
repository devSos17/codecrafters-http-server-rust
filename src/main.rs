use std::{
    io::Write,
    net::{SocketAddr, TcpListener, TcpStream},
};

fn main() {
    let addr = SocketAddr::from(([0, 0, 0, 0], 4221));
    let listener = TcpListener::bind(addr).unwrap();
    println!("Listening from {}", addr);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_client(stream),
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_client(mut stream: TcpStream) {
    let bod: Option<ResponseBody> = Some(ResponseBody::create("Hola mundo".to_string()));
    let res: Response = Response::create(HttpStatus::OK, None, bod);
    println!("{}-{}", res.status.value(), stream.peer_addr().unwrap());
    if let Err(e) = stream.write_all(&res.bvalue()) {
        println!("Epic fail:{}", e);
    }
}

enum HttpVersion {
    HTTP1_1,
}

impl HttpVersion {
    fn value(&self) -> &str {
        match *self {
            HttpVersion::HTTP1_1 => "HTTP/1.1",
        }
    }
}

/* from [RFC 2616](https://www.rfc-editor.org/rfc/rfc2616) */
enum HttpStatus {
    Continue,
    SwitchingProtocols,
    OK,
    Created,
    Accepted,
    NonAuthoritativeInformation,
    NoContent,
    ResetContent,
    PartialContent,
    MultipleChoices,
    MovedPermanently,
    Found,
    SeeOther,
    NotModified,
    UseProxy,
    TemporaryRedirect,
    BadRequest,
    Unauthorized,
    PaymentRequired,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    NotAcceptable,
    ProxyAuthenticationRequired,
    RequestTimeout,
    Conflict,
    Gone,
    LengthRequired,
    PreconditionFailed,
    RequestEntityTooLarge,
    RequestURITooLarge,
    UnsupportedMediaType,
    Requestedrangenotsatisfiable,
    ExpectationFailed,
    InternalServerError,
    NotImplemented,
    BadGateway,
    ServiceUnavailable,
    GatewayTimeout,
    HTTPVersionNotSupported,
}

impl HttpStatus {
    fn value(&self) -> (u16, &str) {
        match *self {
            HttpStatus::Continue => (100, "Continue"),
            HttpStatus::SwitchingProtocols => (101, "Switching Protocols"),
            HttpStatus::OK => (200, "OK"),
            HttpStatus::Created => (201, "Created"),
            HttpStatus::Accepted => (202, "Accepted"),
            HttpStatus::NonAuthoritativeInformation => (203, "Non-Authoritative Information"),
            HttpStatus::NoContent => (204, "No Content"),
            HttpStatus::ResetContent => (205, "Reset Content"),
            HttpStatus::PartialContent => (206, "Partial Content"),
            HttpStatus::MultipleChoices => (300, "Multiple Choices"),
            HttpStatus::MovedPermanently => (301, "Moved Permanently"),
            HttpStatus::Found => (302, "Found"),
            HttpStatus::SeeOther => (303, "See Other"),
            HttpStatus::NotModified => (304, "Not Modified"),
            HttpStatus::UseProxy => (305, "Use Proxy"),
            HttpStatus::TemporaryRedirect => (307, "Temporary Redirect"),
            HttpStatus::BadRequest => (400, "Bad Request"),
            HttpStatus::Unauthorized => (401, "Unauthorized"),
            HttpStatus::PaymentRequired => (402, "Payment Required"),
            HttpStatus::Forbidden => (403, "Forbidden"),
            HttpStatus::NotFound => (404, "Not Found"),
            HttpStatus::MethodNotAllowed => (405, "Method Not Allowed"),
            HttpStatus::NotAcceptable => (406, "Not Acceptable"),
            HttpStatus::ProxyAuthenticationRequired => (407, "Proxy Authentication Required"),
            HttpStatus::RequestTimeout => (408, "Request Time-out"),
            HttpStatus::Conflict => (409, "Conflict"),
            HttpStatus::Gone => (410, "Gone"),
            HttpStatus::LengthRequired => (411, "Length Required"),
            HttpStatus::PreconditionFailed => (412, "Precondition Failed"),
            HttpStatus::RequestEntityTooLarge => (413, "Request Entity Too Large"),
            HttpStatus::RequestURITooLarge => (414, "Request-URI Too Large"),
            HttpStatus::UnsupportedMediaType => (415, "Unsupported Media Type"),
            HttpStatus::Requestedrangenotsatisfiable => (416, "Requested range not satisfiable"),
            HttpStatus::ExpectationFailed => (417, "Expectation Failed"),
            HttpStatus::InternalServerError => (500, "Internal Server Error"),
            HttpStatus::NotImplemented => (501, "Not Implemented"),
            HttpStatus::BadGateway => (502, "Bad Gateway"),
            HttpStatus::ServiceUnavailable => (503, "Service Unavailable"),
            HttpStatus::GatewayTimeout => (504, "Gateway Time-out"),
            HttpStatus::HTTPVersionNotSupported => (505, "HTTP Version not supported"),
        }
    }
}

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

struct Header {}
impl Header {
    fn value(&self) -> String {
        todo!()
    }
}
struct ResponseBody {
    content: String,
}

impl ResponseBody {
    fn create(body: String) -> Self {
        ResponseBody { content: body }
    }
    fn value(&self) -> String {
        self.content.clone()
    }
}
struct Response {
    status: StatusLine,
    header: Option<Header>,
    body: Option<ResponseBody>,
}

impl Response {
    fn create(status: HttpStatus, header: Option<Header>, body: Option<ResponseBody>) -> Self {
        Response {
            status: StatusLine::create(status, None),
            header,
            body,
        }
    }

    fn value(&self) -> String {
        let status: String = self.status.value();
        let headers: String = match &self.header {
            Some(header) => header.value(),
            None => "".to_string(),
        };
        let body: String = match &self.body {
            Some(body) => body.value(),
            None => "".to_string(),
        };
        let res: String = format!("{status}\r\n{headers}\r\n{body}");
        res
    }

    fn bvalue(&self) -> Vec<u8> {
        let val: String = self.value();
        val.into_bytes()
    }
}
