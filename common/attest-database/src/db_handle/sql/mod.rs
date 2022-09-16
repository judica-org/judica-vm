pub use get::*;
pub use insert::*;
pub use setup::*;
pub use update::*;
pub mod insert {
    pub const SQL_INSERT_NONCE_BY_KEY: &str = include_str!("../sql/insert/nonce.sql");
    pub const SQL_INSERT_HIDDEN_SERVICE: &str = include_str!("../sql/insert/hidden_service.sql");
    pub const SQL_INSERT_KEYPAIR: &str = include_str!("../sql/insert/keypair.sql");
    pub const SQL_INSERT_USER: &str = include_str!("../sql/insert/user.sql");
    pub const SQL_INSERT_CHAIN_COMMIT_GROUP: &str =
        include_str!("../sql/insert/new_chain_commit_group.sql");
    pub const SQL_INSERT_CHAIN_COMMIT_GROUP_MEMBER: &str =
        include_str!("../sql/insert/add_chain_commit_group_member.sql");
    pub const SQL_INSERT_CHAIN_COMMIT_GROUP_SUBSCRIBER: &str =
        include_str!("../sql/insert/add_chain_commit_group_subscriber.sql");
    pub const SQL_INSERT_ENVELOPE: &str = include_str!("../sql/insert/envelope.sql");
}

pub mod update {
    pub const SQL_UPDATE_CONNECT_RECURSIVE: &str = include_str!("../sql/update/do_connect.sql");
    pub const SQL_UPDATE_HIDDEN_SERVICE: &str = include_str!("../sql/update/hidden_service.sql");
    pub const SQL_UPDATE_CONNECT_PARENTS: &str = include_str!("../sql/update/resolve_prev_ids.sql");
}

pub mod get {
    pub use chain_commit_groups::*;
    pub use hidden_services::*;
    pub use messages::*;
    pub use nonces::*;
    pub use users::*;
    pub mod chain_commit_groups {
        pub const SQL_GET_ALL_CHAIN_COMMIT_GROUPS: &str =
            include_str!("../sql/get/chain_commit_groups/all_chain_commit_groups.sql");
        pub const SQL_GET_ALL_CHAIN_COMMIT_GROUPS_FOR_CHAIN: &str =
            include_str!("../sql/get/chain_commit_groups/all_chain_commit_groups_for_chain.sql");
        pub const SQL_GET_ALL_CHAIN_COMMIT_GROUP_MEMBERS_FOR_CHAIN: &str = include_str!(
            "../sql/get/chain_commit_groups/all_chain_commit_group_members_for_chain.sql"
        );
        pub const SQL_GET_ALL_CHAIN_COMMIT_GROUP_MEMBERS_TIPS_FOR_CHAIN: &str = include_str!(
            "../sql/get/chain_commit_groups/all_chain_commit_group_members_tips_for_chain.sql"
        );
    }
    pub mod hidden_services {

        pub const SQL_GET_ALL_HIDDEN_SERVICES: &str =
            include_str!("../sql/get/hidden_services/all.sql");
    }

    pub mod messages {

        pub const SQL_GET_MESSAGES_NEWER_THAN_FOR_GENESIS: &str =
            include_str!("../sql/get/connected_messages_newer_than_for_genesis.sql");
        pub const SQL_GET_MESSAGES_BY_HEIGHT_AND_USER: &str =
            include_str!("../sql/get/message_by_height_and_user.sql");
        pub const SQL_GET_MESSAGES_TIPS_BY_USER: &str =
            include_str!("../sql/get/message_tips_by_user.sql");
        pub const SQL_GET_TIPS_FOR_KNOWN_KEYS: &str =
            include_str!("../sql/get/tips_for_known_keys.sql");
        pub const SQL_GET_DISCONNECTED_TIPS_FOR_KNOWN_KEYS: &str =
            include_str!("../sql/get/disconnected_tips_for_known_keys.sql");
        pub const SQL_GET_ALL_MESSAGES_AFTER_CONNECTED: &str =
            include_str!("../sql/get/all_messages_after_connected.sql");
        pub const SQL_GET_ALL_MESSAGES_CONNECTED: &str =
            include_str!("../sql/get/all_messages_connected.sql");
        pub const SQL_GET_ALL_MESSAGES_AFTER_INCONSISTENT: &str =
            include_str!("../sql/get/all_messages_after.sql");
        pub const SQL_GET_ALL_MESSAGES_INCONSISTENT: &str =
            include_str!("../sql/get/all_messages.sql");
        pub const SQL_GET_ALL_TIPS_FOR_ALL_USERS: &str =
            include_str!("../sql/get/all_tips_for_all_users.sql");
        pub const SQL_GET_ALL_GENESIS: &str = include_str!("../sql/get/all_genesis.sql");
        pub const SQL_GET_ALL_MESSAGES_BY_KEY_CONNECTED: &str =
            include_str!("../sql/get/all_messages_by_key_connected.sql");
        pub const SQL_GET_MESSAGE_EXISTS: &str = include_str!("../sql/get/messages/exists.sql");
        pub const SQL_GET_MESSAGE_BY_HASH: &str = include_str!("../sql/get/messages/by_hash.sql");
        pub const SQL_GET_MESSAGE_BY_ID: &str = include_str!("../sql/get/messages/by_id.sql");
    }
    pub mod nonces {

        pub const SQL_GET_SECRET_FOR_NONCE: &str =
            include_str!("../sql/get/nonces/secret_for_nonce.sql");
        pub const SQL_GET_REUSED_NONCE: &str =
            include_str!("../sql/get/nonces/reused_nonces.sql");
    }
    pub mod users {

        pub const SQL_GET_ALL_USERS: &str = include_str!("../sql/get/users/all_users.sql");
        pub const SQL_GET_USER_BY_KEY: &str =
            include_str!("../sql/get/users/user_by_key.sql");
        pub const SQL_GET_ALL_SECRET_KEYS: &str =
            include_str!("../sql/get/users/all_secret_keys.sql");
    }
}
pub mod setup {
    pub const SQL_CREATE_TABLES: &str = concat!(
        "PRAGMA foreign_keys = ON;",
        include_str!("../sql/tables/users.sql"),
        include_str!("../sql/tables/messages.sql"),
        include_str!("../sql/tables/nonces.sql"),
        include_str!("../sql/tables/private_keys.sql"),
        include_str!("../sql/tables/chain_commit_groups.sql"),
        include_str!("../sql/tables/chain_commit_group_members.sql"),
        include_str!("../sql/tables/chain_commit_group_subscribers.sql"),
        include_str!("../sql/tables/hidden_services.sql"),
        include_str!("../sql/triggers/messages/connect_gap_parent.sql"),
        "PRAGMA journal_mode = WAL;"
    );
}

pub const CACHED: &[&str] = &[
    SQL_INSERT_NONCE_BY_KEY,
    SQL_INSERT_HIDDEN_SERVICE,
    SQL_INSERT_KEYPAIR,
    SQL_INSERT_USER,
    SQL_INSERT_CHAIN_COMMIT_GROUP,
    SQL_INSERT_CHAIN_COMMIT_GROUP_MEMBER,
    SQL_INSERT_CHAIN_COMMIT_GROUP_SUBSCRIBER,
    SQL_INSERT_ENVELOPE,
    SQL_UPDATE_CONNECT_RECURSIVE,
    SQL_UPDATE_HIDDEN_SERVICE,
    SQL_UPDATE_CONNECT_PARENTS,
    SQL_GET_ALL_CHAIN_COMMIT_GROUPS,
    SQL_GET_ALL_CHAIN_COMMIT_GROUPS_FOR_CHAIN,
    SQL_GET_ALL_CHAIN_COMMIT_GROUP_MEMBERS_FOR_CHAIN,
    SQL_GET_ALL_CHAIN_COMMIT_GROUP_MEMBERS_TIPS_FOR_CHAIN,
    SQL_GET_ALL_HIDDEN_SERVICES,
    SQL_GET_MESSAGES_NEWER_THAN_FOR_GENESIS,
    SQL_GET_MESSAGES_BY_HEIGHT_AND_USER,
    SQL_GET_MESSAGES_TIPS_BY_USER,
    SQL_GET_TIPS_FOR_KNOWN_KEYS,
    SQL_GET_DISCONNECTED_TIPS_FOR_KNOWN_KEYS,
    SQL_GET_ALL_MESSAGES_AFTER_CONNECTED,
    SQL_GET_ALL_MESSAGES_CONNECTED,
    SQL_GET_ALL_MESSAGES_AFTER_INCONSISTENT,
    SQL_GET_ALL_MESSAGES_INCONSISTENT,
    SQL_GET_ALL_TIPS_FOR_ALL_USERS,
    SQL_GET_ALL_GENESIS,
    SQL_GET_ALL_MESSAGES_BY_KEY_CONNECTED,
    SQL_GET_MESSAGE_EXISTS,
    SQL_GET_MESSAGE_BY_HASH,
    SQL_GET_MESSAGE_BY_ID,
    SQL_GET_SECRET_FOR_NONCE,
    SQL_GET_REUSED_NONCE,
    SQL_GET_ALL_USERS,
    SQL_GET_USER_BY_KEY,
    SQL_GET_ALL_SECRET_KEYS,
];
