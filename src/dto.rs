use serde::Serialize;

/// Top-level category (first-level group under root).
#[derive(Serialize)]
pub struct CategoryDto {
    pub id: String,
    pub name: String,
}

/// Summary info about a thread within a category.
#[derive(Serialize)]
pub struct ThreadSummaryDto {
    pub id: String,
    pub title: String,
    pub post_count: usize,
}

/// A single post (entry) inside a thread.
#[derive(Serialize)]
pub struct PostDto {
    pub id: String,
    pub title: String,
    pub author: String,
    pub body: String,
}

/// Full thread detail with all posts.
#[derive(Serialize)]
pub struct ThreadDetailDto {
    pub id: String,
    pub title: String,
    pub posts: Vec<PostDto>,
}

