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

#[derive(Debug)]
pub struct LockFailed {
    pub name: String,
}
impl LockFailed {
    pub fn new(name: String) -> LockFailed {
        LockFailed { name }
    }
}
impl std::fmt::Display for LockFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lock Failed: {}", self.name)
    }
}
impl std::error::Error for LockFailed {}
