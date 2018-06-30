#[macro_use]
extern crate kubeless;

fn say_hello(event: kubeless::Event, ctx: kubeless::Context) -> String {
    match event.data {
        Some(name) => format!("Hello, {}", String::from_utf8_lossy(&name)),
        None => String::from("Hello"),
    }
}

fn say_goodbye(event: kubeless::Event, ctx: kubeless::Context) -> String {
    match event.data {
        Some(name) => format!("Goodbye, {}", String::from_utf8_lossy(&name)),
        None => String::from("Goodbye"),
    }
}

fn echo_or_panic(event: kubeless::Event, ctx: kubeless::Context) -> String {
    String::from_utf8_lossy(&event.data.unwrap()).to_string()
}

fn main() {
    kubeless::start(select_function!(say_hello, say_goodbye, echo_or_panic));
}
