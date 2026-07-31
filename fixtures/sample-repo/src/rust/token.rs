use std::collections::HashMap;

mod helpers;

pub fn issue_token(user: &str) -> String {
    let salt = helper_salt();
    format!("{user}:{salt}")
}

fn helper_salt() -> String {
    "s3cret".into()
}

pub async fn refresh_token(token: &str) -> String {
    issue_token(token)
}
