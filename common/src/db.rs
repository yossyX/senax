/// Convert MySQL placeholder (?) to PostgreSQL placeholders ($1, $2, $3, ...)
///
/// This function efficiently converts MySQL-style question mark placeholders to PostgreSQL-style
/// numbered placeholders while respecting SQL string literals and comments.
///
/// # Examples
///
/// ```rust
/// use senax_pgsql_parser::convert_mysql_placeholders_to_postgresql;
///
/// let mysql_sql = "SELECT * FROM users WHERE id = ? AND name = ?";
/// let postgresql_sql = convert_mysql_placeholders_to_postgresql(mysql_sql);
/// assert_eq!(postgresql_sql, "SELECT * FROM users WHERE id = $1 AND name = $2");
///
/// // String literals are preserved
/// let mysql_sql = "SELECT * FROM users WHERE name = 'user?' AND id = ?";
/// let postgresql_sql = convert_mysql_placeholders_to_postgresql(mysql_sql);
/// assert_eq!(postgresql_sql, "SELECT * FROM users WHERE name = 'user?' AND id = $1");
/// ```
pub fn convert_mysql_placeholders_to_postgresql(sql: &str) -> String {
    let mut result = String::with_capacity(sql.len() + 32); // Pre-allocate with extra space for placeholders
    let mut chars = sql.chars().peekable();
    let mut placeholder_count = 0;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while let Some(ch) = chars.next() {
        match ch {
            // Handle string literals
            '\'' if !in_double_quote && !in_line_comment && !in_block_comment => {
                in_single_quote = !in_single_quote;
                result.push(ch);
            }
            '"' if !in_single_quote && !in_line_comment && !in_block_comment => {
                in_double_quote = !in_double_quote;
                result.push(ch);
            }

            // Handle SQL comments
            '-' if !in_single_quote && !in_double_quote && !in_block_comment => {
                if chars.peek() == Some(&'-') {
                    chars.next(); // consume second '-'
                    in_line_comment = true;
                    result.push_str("--");
                } else {
                    result.push(ch);
                }
            }
            '/' if !in_single_quote && !in_double_quote && !in_line_comment => {
                if chars.peek() == Some(&'*') {
                    chars.next(); // consume '*'
                    in_block_comment = true;
                    result.push_str("/*");
                } else {
                    result.push(ch);
                }
            }
            '*' if in_block_comment => {
                if chars.peek() == Some(&'/') {
                    chars.next(); // consume '/'
                    in_block_comment = false;
                    result.push_str("*/");
                } else {
                    result.push(ch);
                }
            }
            '\n' | '\r' if in_line_comment => {
                in_line_comment = false;
                result.push(ch);
            }

            // Handle escape sequences in string literals
            '\\' if (in_single_quote || in_double_quote)
                && !in_line_comment
                && !in_block_comment =>
            {
                result.push(ch);
                if let Some(next_ch) = chars.next() {
                    result.push(next_ch); // Push the escaped character as-is
                }
            }

            // Handle placeholder conversion
            '?' if !in_single_quote
                && !in_double_quote
                && !in_line_comment
                && !in_block_comment =>
            {
                placeholder_count += 1;
                result.push('$');
                result.push_str(&placeholder_count.to_string());
            }

            // Default case: just push the character
            _ => {
                result.push(ch);
            }
        }
    }

    result
}
