pub mod cache;
pub mod err;
#[cfg(feature = "etcd")]
pub mod etcd;
pub mod fulltext;
pub mod linker;
pub mod session;
pub mod types {
    pub mod blob;
    pub mod geo_point;
    pub mod point;
}
pub mod db;
pub use db::convert_mysql_placeholders_to_postgresql;
pub mod update_operator;

pub type ShardId = u16;

#[macro_export]
macro_rules! if_then_else {
    ( $if:expr, $then:expr, $else:expr ) => {
        if $if { $then } else { $else }
    };
}

pub trait SqlColumns {
    fn _sql_cols() -> &'static str;
}
