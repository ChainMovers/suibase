// Standalone functions to manipulate strings
pub fn remove_ascii_color_code(s: &str) -> String {
    let mut result = String::new();
    let mut is_color_code = false;
    for c in s.chars() {
        if is_color_code {
            if c == 'm' {
                is_color_code = false;
            }
        } else if c == '\x1b' {
            is_color_code = true;
        } else {
            result.push(c);
        }
    }
    result
}
