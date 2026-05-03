// Multiple function shapes: free fn + impl method + trait default impl.

pub struct Counter {
    pub n: u32,
}

impl Counter {
    pub fn new() -> Self {
        Self { n: 0 }
    }

    pub fn bump(&mut self) {
        self.n += 1;
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Greeter {
    fn name(&self) -> &str;

    // trait default impl — should be extracted as a function.
    fn greet(&self) -> String {
        format!("hello, {}", self.name())
    }
}

pub fn free_function(a: u32, b: u32) -> u32 {
    a + b
}
