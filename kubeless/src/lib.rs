//! A Library for Writing [Kubeless](https://kubeless.io) Functions
//!
//! ```rust
//! #[macro_use]
//! extern crate kubeless;
//!
//! fn say_hello(event: kubeless::Event, ctx: kubeless::Context) -> String {
//!     String::from("Hello")
//! }
//!
//! fn say_goodbye(event: kubeless::Event, ctx: kubeless::Context) -> String {
//!     String::from("Goodbye")
//! }
//!
//! fn main() {
//!     // Expose say_hello and say_goodbye to Kubeless
//!     kubeless::start(select_function!(say_hello, say_goodbye));
//! }
//! ```

extern crate actix_web;
extern crate bytes;
extern crate futures;

#[macro_use]
extern crate prometheus;

#[macro_use]
extern crate lazy_static;

use actix_web::http::Method;
use actix_web::{server, App, AsyncResponder, HttpMessage, HttpRequest, HttpResponse, Responder};

use futures::{Future, Stream};

pub mod types;
pub use types::*;

/// The default timeout for user functions
pub const DEFAULT_TIMEOUT: usize = 180;

/// The default memory limit
///
/// Currently `DEFAULT_MEMORY_LIMIT` is set to `0` to indicate that no limit was provided
pub const DEFAULT_MEMORY_LIMIT: usize = 0;

lazy_static! {
    // environment variables

    /// The value of the `FUNC_HANDLER` environment variable
    ///
    /// This variable usually isn't used directly and is accessed indirectly via [select_function!](select_function)
    pub static ref FUNC_HANDLER : String = std::env::var("FUNC_HANDLER").expect("the FUNC_HANDLER environment variable must be provided");

    static ref FUNC_TIMEOUT : usize = match std::env::var("FUNC_TIMEOUT") {
        Ok(timeout_str) => timeout_str.parse::<usize>().unwrap_or(DEFAULT_TIMEOUT),
        Err(_) => DEFAULT_TIMEOUT,
    };

    static ref FUNC_RUNTIME : String = std::env::var("FUNC_RUNTIME").unwrap_or_else(|_| String::new());

    static ref FUNC_MEMORY_LIMIT : usize = match std::env::var("FUNC_MEMORY_LIMIT") {
        Ok(mem_limit_str) => mem_limit_str.parse::<usize>().unwrap_or(DEFAULT_MEMORY_LIMIT),
        Err(_) => DEFAULT_MEMORY_LIMIT,
    };

    // metric logging variables
    static ref CALL_HISTOGRAM : prometheus::Histogram = register_histogram!(histogram_opts!("function_duration_seconds", "Duration of user function in seconds")).unwrap();
    static ref CALL_TOTAL : prometheus::Counter = register_counter!(opts!("function_calls_total", "Number of calls to user function")).unwrap();
    // static ref FAILURES_TOTAL : prometheus::Counter = register_counter!(opts!("function_failures_total", "Number of failed calls")).unwrap();
}

#[macro_export]
/// Given a list of functions, return a function with an identifier matching the `FUNC_HANDLER` environment variable
macro_rules! select_function {
    ( $( $x:ident ),* ) => {
        {
            use kubeless::types::UserFunction;
            use kubeless::FUNC_HANDLER;

            let mut selected_function : Option<UserFunction> = None;
            $(
                if stringify!($x) == *FUNC_HANDLER {
                    selected_function = Some($x);
                }
            )*

            match selected_function {
                Some(result) => result,
                None => {
                    let mut available_functions = String::new();
                    $(
                        if available_functions.len() > 0 {
                            available_functions.push_str(", ");
                        }
                        available_functions.push_str(stringify!($x));
                    )*
                    panic!("No function named {} available, available functions are: {}", *FUNC_HANDLER, available_functions)
                }
            }
        }
    };
}

// TODO per https://kubeless.io/docs/implementing-new-runtime/#2-1-additional-features this should return 408 even though I can't kill the function in a sane way
fn handle_request(
    req: HttpRequest,
    user_function: UserFunction,
) -> Box<Future<Item = HttpResponse, Error = actix_web::Error>> {
    let get_header = |req: &HttpRequest, header_name: &str| match req.headers().get(header_name) {
        Some(header) => header
            .to_str()
            .map(String::from)
            .unwrap_or_else(|_| String::new()),
        None => String::new(),
    };

    let event_id = get_header(&req, "event-id");
    let event_type = get_header(&req, "event-type");
    let event_time = get_header(&req, "event-time");
    let event_namespace = get_header(&req, "event-namespace");

    let body_future: Box<Future<Item = Option<bytes::Bytes>, Error = actix_web::Error>> =
        if req.method() == &Method::POST {
            Box::new(
                req.concat2()
                    .from_err()
                    .map(move |bytes: bytes::Bytes| Some(bytes)),
            )
        } else {
            Box::new(futures::future::ok::<Option<bytes::Bytes>, actix_web::Error>(None))
        };

    body_future
        .map(move |data: Option<bytes::Bytes>| {
            CALL_TOTAL.inc();
            let timer = CALL_HISTOGRAM.start_timer();

            let call_event = Event {
                data,
                event_id,
                event_type,
                event_time,
                event_namespace,
            };

            let call_context = Context {
                function_name: FUNC_HANDLER.clone(),
                runtime: FUNC_RUNTIME.clone(),
                timeout: *FUNC_TIMEOUT,
                memory_limit: *FUNC_MEMORY_LIMIT,
            };

            let result = user_function(call_event, call_context);
            // TODO figure out how to catch and unwind a panic

            timer.observe_duration();

            HttpResponse::Ok().body(result)
        })
        .responder()
}

/// Handle requests to /healthz
fn healthz(req: HttpRequest) -> impl Responder {
    match req.method() {
        // Accept GET and HEAD requests
        &Method::GET | &Method::HEAD => HttpResponse::Ok().content_type("plain/text").body("OK"),
        // Reject any other type of request with 400 Bad Request
        _ => HttpResponse::BadRequest().body("Bad Request"),
    }
}

/// Start the HTTP server that Kubeless will use to interact with the container running this code
pub fn start(func: UserFunction) {
    let port = std::env::var("FUNC_PORT").unwrap_or_else(|_| String::from("8080"));
    server::new(move || {
        App::new()
            .resource("/", move |r| r.f(move |req| handle_request(req, func)))
            .resource("/healthz", |r| r.f(healthz))
    }).bind(format!("127.0.0.1:{}", &port))
        .expect(&format!("Can not bind to port {}", &port))
        .run();
}
