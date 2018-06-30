use bytes::Bytes;

/// Contains information about the call to the user function
pub struct Event {
    /// The data passed to the user function in the request
    pub data: Option<Bytes>,

    /// TODO document
    pub event_id: String,

    /// TODO document
    pub event_type: String,

    /// TODO document
    pub event_time: String,

    /// TODO document
    pub event_namespace: String,
}

/// Contains information about the environment
pub struct Context {
    pub function_name: String,
    pub runtime: String,
    pub timeout: usize,
    pub memory_limit: usize,
}

/// A function callable from kubeless
pub type UserFunction = fn(event: Event, context: Context) -> String;
