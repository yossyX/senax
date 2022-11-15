#[derive(Debug)]
pub struct RowNotFound {
    pub table: &'static str,
    pub id: String,
}
impl RowNotFound {
    pub fn new(table: &'static str, id: String) -> RowNotFound {
        RowNotFound { table, id }
    }
}
impl std::fmt::Display for RowNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Row not found in {}: {}", self.table, self.id)
    }
}
impl std::error::Error for RowNotFound {}
