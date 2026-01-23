//! Test data generation utilities for benchmarks.
//!
//! Provides functions to generate text of various sizes and patterns
//! for performance testing.

use rand::Rng;

/// Generates a realistic Rust source file of approximately `target_bytes` size.
pub fn generate_rust_source(target_bytes: usize) -> String {
    let mut content = String::with_capacity(target_bytes);
    let mut rng = rand::thread_rng();

    // File header
    content.push_str("//! Auto-generated Rust source for benchmarking.\n\n");
    content.push_str("use std::collections::HashMap;\n");
    content.push_str("use std::sync::Arc;\n\n");

    let mut struct_count = 0;
    let mut func_count = 0;

    while content.len() < target_bytes {
        // Randomly add different code constructs
        let choice: u32 = rng.r#gen();
        match choice % 5 {
            0 => {
                // Add a struct
                struct_count += 1;
                content.push_str(&format!(
                    r#"/// Documentation for struct {0}.
#[derive(Debug, Clone)]
pub struct Benchmark{0} {{
    pub id: u64,
    pub name: String,
    pub values: Vec<f64>,
    pub metadata: HashMap<String, String>,
}}

impl Benchmark{0} {{
    pub fn new(id: u64, name: &str) -> Self {{
        Self {{
            id,
            name: name.to_string(),
            values: Vec::new(),
            metadata: HashMap::new(),
        }}
    }}

    pub fn add_value(&mut self, value: f64) {{
        self.values.push(value);
    }}
}}

"#,
                    struct_count
                ));
            }
            1 => {
                // Add a function
                func_count += 1;
                content.push_str(&format!(
                    r#"/// Processes data for benchmark {0}.
pub fn process_data_{0}(input: &[u8], multiplier: f64) -> Result<Vec<f64>, String> {{
    let mut results = Vec::with_capacity(input.len());

    for (i, &byte) in input.iter().enumerate() {{
        let value = (byte as f64) * multiplier;
        if value.is_nan() {{
            return Err(format!("Invalid value at index {{}}", i));
        }}
        results.push(value);
    }}

    Ok(results)
}}

"#,
                    func_count
                ));
            }
            2 => {
                // Add an enum
                let n: u32 = rng.r#gen();
                content.push_str(&format!(
                    r#"/// Status enum for benchmark operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchmarkStatus{} {{
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}}

"#,
                    n % 1000
                ));
            }
            3 => {
                // Add a trait impl
                content.push_str(&format!(
                    r#"impl Default for Benchmark{} {{
    fn default() -> Self {{
        Self::new(0, "default")
    }}
}}

"#,
                    if struct_count > 0 { struct_count } else { 1 }
                ));
            }
            4 => {
                // Add a const and some comments
                let n: u32 = rng.r#gen();
                content.push_str(&format!(
                    r#"/// Maximum buffer size for benchmark {0}.
pub const MAX_BUFFER_SIZE_{0}: usize = 1024 * 1024;

// TODO: Optimize this for larger datasets
// FIXME: Handle edge cases properly
// NOTE: This is performance-critical code

"#,
                    n % 10000
                ));
            }
            _ => {}
        }
    }

    // Trim to exact size
    content.truncate(target_bytes);
    content
}

/// Generates plain text of approximately `target_bytes` size.
pub fn generate_plain_text(target_bytes: usize) -> String {
    let mut content = String::with_capacity(target_bytes);
    let words = [
        "the", "quick", "brown", "fox", "jumps", "over", "lazy", "dog",
        "lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing",
        "performance", "benchmark", "editor", "text", "buffer", "syntax",
        "highlighting", "rendering", "viewport", "cursor", "selection",
    ];
    let mut rng = rand::thread_rng();

    while content.len() < target_bytes {
        // Add 5-15 words per line
        let words_in_line: usize = (rng.r#gen::<usize>() % 10) + 5;
        for i in 0..words_in_line {
            if i > 0 {
                content.push(' ');
            }
            let idx: usize = rng.r#gen::<usize>() % words.len();
            content.push_str(words[idx]);
        }
        content.push('\n');
    }

    content.truncate(target_bytes);
    content
}

/// Generates JSON content of approximately `target_bytes` size.
pub fn generate_json(target_bytes: usize) -> String {
    let mut content = String::with_capacity(target_bytes);
    content.push_str("{\n");
    content.push_str("  \"benchmark_data\": [\n");

    let mut rng = rand::thread_rng();
    let mut i = 0;

    while content.len() < target_bytes - 100 {
        if i > 0 {
            content.push_str(",\n");
        }
        let val: f64 = rng.r#gen::<f64>() * 1000.0;
        let active: bool = rng.r#gen();
        content.push_str(&format!(
            r#"    {{
      "id": {},
      "name": "item_{}",
      "value": {:.6},
      "active": {},
      "tags": ["benchmark", "performance", "test"]
    }}"#,
            i,
            i,
            val,
            if active { "true" } else { "false" }
        ));
        i += 1;
    }

    content.push_str("\n  ]\n}\n");
    content
}

/// Returns approximate byte sizes for different test scenarios.
pub struct TestSizes;

impl TestSizes {
    pub const TINY: usize = 1_000;          // 1 KB - trivial file
    pub const SMALL: usize = 10_000;        // 10 KB - typical source file
    pub const MEDIUM: usize = 100_000;      // 100 KB - large source file
    pub const LARGE: usize = 1_000_000;     // 1 MB - very large file
    pub const HUGE: usize = 10_000_000;     // 10 MB - stress test
}

/// Generates typing input for throughput tests.
pub fn generate_typing_input(char_count: usize) -> Vec<char> {
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 \n"
        .chars()
        .collect();

    (0..char_count)
        .map(|_| {
            let idx: usize = rng.r#gen::<usize>() % chars.len();
            chars[idx]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_source_size() {
        let content = generate_rust_source(10_000);
        assert!(content.len() >= 9_000 && content.len() <= 10_000);
    }

    #[test]
    fn test_plain_text_size() {
        let content = generate_plain_text(10_000);
        assert!(content.len() >= 9_000 && content.len() <= 10_000);
    }

    #[test]
    fn test_json_size() {
        let content = generate_json(10_000);
        assert!(content.len() >= 9_000);
    }
}
