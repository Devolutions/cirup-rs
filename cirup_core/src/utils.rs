pub fn sanitized(component: &str) -> String {
    let mut buf = String::with_capacity(component.len());
    for (i, c) in component.chars().enumerate() {
        let is_lower = 'a' <= c && c <= 'z';
        let is_upper = 'A' <= c && c <= 'Z';
        let is_letter = is_upper || is_lower;
        let is_number = '0' <= c && c <= '9';
        let is_space = c == ' ';
        let is_hyphen = c == '-';
        let is_underscore = c == '_';
        let is_period = c == '.' && i != 0; // Disallow accidentally hidden folders
        let is_valid =
            is_letter || is_number || is_space || is_hyphen || is_underscore || is_period;
        if is_valid {
            buf.push(c);
        }
    }
    buf
}
