use crate::identity::UserAddress;

pub fn admin_message_prefix(user: UserAddress) -> String {
    format!("admin/{user}")
}

pub fn admin_set_moderator_message_prefix(user: UserAddress) -> String {
    format!("moderator/{user}")
}

pub fn admin_set_server_message_prefix(user: UserAddress) -> String {
    format!("set_server/{user}")
}
