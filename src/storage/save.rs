use std::fs;
use serde_json;

use crate::storage::context::Context;

pub fn save_context(ctx: &Context) {

    let dir = ".git/commitlens";

    fs::create_dir_all(dir).unwrap();

    let file = format!("{}/{}.json", dir, ctx.commit);

    let json = serde_json::to_string_pretty(ctx).unwrap();

    fs::write(file, json).unwrap();
}