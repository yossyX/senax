use serde::{Deserialize, Serialize};

#[derive(Eq, PartialEq, Copy, Clone, Deserialize, Serialize)]
pub enum Role {
    Admin,
    User,
    Guest,
}
@{-"\n"}@