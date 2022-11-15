pub mod err;
pub mod types {
    pub mod blob;
    pub mod point;
}
pub mod cache;
pub mod linker;

pub type ShardId = u16;

#[macro_export]
macro_rules! if_then_else {
    ( $if:expr, $then:expr, $else:expr ) => {
        if $if {
            $then
        } else {
            $else
        }
    };
}

pub trait SqlColumns {
    fn _sql_cols(quote: &'static str) -> &'static str;
}
