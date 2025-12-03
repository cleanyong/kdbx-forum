use std::{error::Error, fs::File, path::PathBuf};

use keepass::{
    db::{Entry, Group, Node, NodeRef, Value},
    Database, DatabaseKey,
};
use rpassword::prompt_password;

use crate::dto::PostDto;

/// Build a DatabaseKey from optional password and keyfile path.
pub fn build_db_key(
    password_arg: Option<String>,
    keyfile: &Option<PathBuf>,
) -> Result<DatabaseKey, Box<dyn Error>> {
    let password = match password_arg {
        Some(p) => p,
        None => prompt_password("Master password: ")?,
    };

    let mut key = DatabaseKey::new().with_password(&password);
    if let Some(keyfile_path) = keyfile {
        let mut keyfile = File::open(keyfile_path)?;
        key = key.with_keyfile(&mut keyfile)?;
    }

    Ok(key)
}

/// Open and decrypt the KeePass database from disk.
pub fn open_database(
    path: &PathBuf,
    key: &DatabaseKey,
) -> Result<Database, Box<dyn Error>> {
    let mut db_file = File::open(path)?;
    let db = Database::open(&mut db_file, key.clone())?;
    Ok(db)
}

/// Recursively count all entries under a group (including nested groups).
pub fn count_entries_in_group(group: &Group) -> usize {
    let mut count = 0;
    for node in &group.children {
        match node.as_ref() {
            NodeRef::Entry(_) => count += 1,
            NodeRef::Group(g) => count += count_entries_in_group(g),
        }
    }
    count
}

/// Convert an Entry into a PostDto.
pub fn entry_to_post_dto(entry: &Entry) -> PostDto {
    let title = entry.get_title().unwrap_or("").to_string();
    let author = entry.get_username().unwrap_or("").to_string();
    let body = entry.get("Notes").unwrap_or("").to_string();

    PostDto {
        id: entry.uuid.to_string(),
        title,
        author,
        body,
    }
}

/// Recursively find a group by its UUID (string form) starting from `group`.
pub fn find_group_by_id<'a>(group: &'a Group, id: &str) -> Option<&'a Group> {
    if group.uuid.to_string() == id {
        return Some(group);
    }

    for node in &group.children {
        if let NodeRef::Group(g) = node.as_ref() {
            if let Some(found) = find_group_by_id(g, id) {
                return Some(found);
            }
        }
    }

    None
}

/// Mutable variant of find_group_by_id.
pub fn find_group_by_id_mut<'a>(group: &'a mut Group, id: &str) -> Option<&'a mut Group> {
    if group.uuid.to_string() == id {
        return Some(group);
    }

    for node in &mut group.children {
        if let Node::Group(g) = node {
            if let Some(found) = find_group_by_id_mut(g, id) {
                return Some(found);
            }
        }
    }

    None
}

/// Add a new thread (group + initial post entry) under the given category.
/// Returns the new thread group's UUID as a string.
pub fn add_thread_to_category(
    db: &mut Database,
    category_id: &str,
    title: &str,
    author: &str,
    body: &str,
) -> Result<String, String> {
    let category = find_group_by_id_mut(&mut db.root, category_id)
        .ok_or_else(|| "Category not found".to_string())?;

    let mut thread_group = Group::new(title);

    let mut entry = Entry::new();
    entry
        .fields
        .insert("Title".to_string(), Value::Unprotected(title.to_string()));
    entry
        .fields
        .insert("UserName".to_string(), Value::Unprotected(author.to_string()));
    entry
        .fields
        .insert("Notes".to_string(), Value::Unprotected(body.to_string()));

    thread_group.add_child(entry);

    let thread_id = thread_group.uuid.to_string();
    category.add_child(thread_group);

    Ok(thread_id)
}

/// Add a reply entry to an existing thread group. Returns the new entry UUID.
pub fn add_reply_to_thread(
    db: &mut Database,
    thread_id: &str,
    author: &str,
    body: &str,
) -> Result<String, String> {
    let thread_group = find_group_by_id_mut(&mut db.root, thread_id)
        .ok_or_else(|| "Thread not found".to_string())?;

    let mut entry = Entry::new();
    let title = if body.len() > 40 {
        format!("Reply: {}...", &body[..40])
    } else {
        format!("Reply: {}", body)
    };

    entry
        .fields
        .insert("Title".to_string(), Value::Unprotected(title));
    entry
        .fields
        .insert("UserName".to_string(), Value::Unprotected(author.to_string()));
    entry
        .fields
        .insert("Notes".to_string(), Value::Unprotected(body.to_string()));

    let id = entry.uuid.to_string();
    thread_group.add_child(entry);

    Ok(id)
}

/// Persist the current in-memory database back to disk safely using a temporary file + rename.
pub fn save_database(
    db: &Database,
    db_path: &PathBuf,
    key: &DatabaseKey,
) -> Result<(), Box<dyn Error>> {
    let tmp_path = db_path.with_extension("kdbx.tmp");
    let mut tmp_file = File::create(&tmp_path)?;
    db.save(&mut tmp_file, key.clone())?;
    std::fs::rename(&tmp_path, db_path)?;
    Ok(())
}
