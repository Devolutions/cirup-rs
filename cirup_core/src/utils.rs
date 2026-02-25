pub(crate) fn sanitized(component: &str) -> String {
    let mut buf = String::with_capacity(component.len());
    for (i, c) in component.chars().enumerate() {
        let is_lower = c.is_ascii_lowercase();
        let is_upper = c.is_ascii_uppercase();
        let is_letter = is_upper || is_lower;
        let is_number = c.is_ascii_digit();
        let is_space = c == ' ';
        let is_hyphen = c == '-';
        let is_underscore = c == '_';
        let is_period = c == '.' && i != 0; // Disallow accidentally hidden folders
        let is_valid = is_letter || is_number || is_space || is_hyphen || is_underscore || is_period;
        if is_valid {
            buf.push(c);
        }
    }
    buf
}
