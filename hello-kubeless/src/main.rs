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

fn main() {
    kubeless::start(select_function!(say_hello, say_goodbye));
}
