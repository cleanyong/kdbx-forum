use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    Json,
};
use keepass::db::NodeRef;
use serde::Deserialize;

use crate::{
    db::{
        add_reply_to_thread, add_thread_to_category, count_entries_in_group, entry_to_post_dto,
        find_group_by_id, save_database,
    },
    dto::{CategoryDto, ThreadDetailDto, ThreadSummaryDto},
    state::AppState,
};

/// Forum frontend page (HTML + JS).
pub async fn index() -> Html<String> {
    let body = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>kdbx-forum</title>
  <style>
    body { font-family: system-ui, sans-serif; max-width: 960px; margin: 2rem auto; display: flex; gap: 1.5rem; }
    code { background: #f5f5f5; padding: 0.1rem 0.3rem; }
    #sidebar { width: 260px; border-right: 1px solid #ddd; padding-right: 1rem; }
    #main { flex: 1; }
    ul { list-style: none; padding-left: 0; }
    li { margin: 0.25rem 0; }
    a { cursor: pointer; color: #0366d6; text-decoration: none; }
    a:hover { text-decoration: underline; }
    textarea { width: 100%; min-height: 5rem; }
    input[type="text"] { width: 100%; }
    .post { border-bottom: 1px solid #eee; padding: 0.5rem 0; }
    .post-author { font-weight: 600; }
    .post-title { font-weight: 600; }
    .post-body { white-space: pre-wrap; margin-top: 0.25rem; }
    .muted { color: #666; font-size: 0.9rem; }
    .thread-unread { font-weight: 600; }
    .thread-read { font-weight: 400; }
  </style>
</head>
<body>
  <div id="sidebar">
    <h2>kdbx-forum</h2>
    <p class="muted">Mini forum backed by a KeePass KDBX file.</p>

    <h3>Your name</h3>
    <input type="text" id="username" placeholder="Anonymous" />
    <p class="muted">Used as the author when you post threads or replies.</p>

    <h3>Categories</h3>
    <ul id="categories"></ul>
  </div>

  <div id="main">
    <section>
      <h2 id="current-category-title">Select a category</h2>
      <ul id="threads"></ul>
    </section>

    <section id="new-thread-section" style="display:none; margin-top:1rem;">
      <h3>New thread in this category</h3>
      <input type="text" id="new-thread-title" placeholder="Thread title" />
      <br><br>
      <textarea id="new-thread-body" placeholder="Thread body"></textarea>
      <br>
      <button id="new-thread-submit">Post thread</button>
      <span id="new-thread-status" class="muted"></span>
    </section>

    <section style="margin-top:2rem;">
      <h2 id="current-thread-title">Thread</h2>
      <div id="thread-posts"></div>

      <div id="reply-section" style="display:none; margin-top:1rem;">
        <h3>Reply</h3>
        <textarea id="reply-body" placeholder="Write your reply here"></textarea>
        <br>
        <button id="reply-submit">Post reply</button>
        <span id="reply-status" class="muted"></span>
      </div>
    </section>
  </div>

  <script>
    let selectedCategoryId = null;
    let selectedThreadId = null;

    function loadReadState() {
      try {
        const raw = localStorage.getItem('kdbx_forum_read_state');
        if (!raw) return {};
        const parsed = JSON.parse(raw);
        return (parsed && typeof parsed === 'object') ? parsed : {};
      } catch (e) {
        console.warn('Failed to parse read state from localStorage', e);
        return {};
      }
    }

    function saveReadState(state) {
      try {
        localStorage.setItem('kdbx_forum_read_state', JSON.stringify(state));
      } catch (e) {
        console.warn('Failed to save read state to localStorage', e);
      }
    }

    function markThreadRead(threadId, postCount) {
      const state = loadReadState();
      state[threadId] = postCount;
      saveReadState(state);
      updateThreadListReadStyles();
    }

    function updateThreadListReadStyles() {
      const state = loadReadState();
      document.querySelectorAll('#threads a[data-thread-id]').forEach(a => {
        const id = a.dataset.threadId;
        const postCount = Number(a.dataset.postCount || '0');
        const lastSeen = state[id] || 0;
        const isRead = lastSeen >= postCount && postCount > 0;
        a.classList.toggle('thread-read', isRead);
        a.classList.toggle('thread-unread', !isRead);
      });
    }

    function getUsername() {
      const stored = localStorage.getItem('kdbx_forum_username') || '';
      const field = document.getElementById('username');
      if (!field.value && stored) {
        field.value = stored;
      }
      return field.value.trim() || 'Anonymous';
    }

    document.getElementById('username').addEventListener('input', (e) => {
      localStorage.setItem('kdbx_forum_username', e.target.value);
    });

    async function loadCategories() {
      const res = await fetch('/categories');
      if (!res.ok) {
        console.error('Failed to load categories:', res.status);
        return;
      }
      const cats = await res.json();
      console.log('Loaded categories from /categories:', cats);
      const ul = document.getElementById('categories');
      ul.innerHTML = '';
      if (!Array.isArray(cats) || cats.length === 0) {
        const li = document.createElement('li');
        li.textContent = '(No categories found in KDBX root)';
        ul.appendChild(li);
        return;
      }
      for (let i = 0; i < cats.length; i++) {
        const cat = cats[i];
        const li = document.createElement('li');
        const a = document.createElement('a');
        a.textContent = cat.name || '(no name)';
        a.onclick = function () { selectCategory(cat); };
        li.appendChild(a);
        ul.appendChild(li);
      }
    }

    async function selectCategory(cat) {
      selectedCategoryId = cat.id;
      selectedThreadId = null;
      document.getElementById('current-category-title').textContent = 'Category: ' + cat.name;
      document.getElementById('threads').innerHTML = '';
      document.getElementById('thread-posts').innerHTML = '';
      document.getElementById('current-thread-title').textContent = 'Thread';
      document.getElementById('reply-section').style.display = 'none';
      document.getElementById('new-thread-section').style.display = 'block';
      document.getElementById('new-thread-status').textContent = '';
      await loadThreads(cat.id);
    }

    async function loadThreads(categoryId) {
      const res = await fetch('/categories/' + encodeURIComponent(categoryId) + '/threads');
      if (!res.ok) {
        console.error('Failed to load threads:', res.status);
        return;
      }
      const threads = await res.json();
      const ul = document.getElementById('threads');
      ul.innerHTML = '';
      if (threads.length === 0) {
        const li = document.createElement('li');
        li.textContent = '(No threads yet in this category)';
        ul.appendChild(li);
        return;
      }
      const readState = loadReadState();
      threads.forEach(th => {
        const li = document.createElement('li');
        const a = document.createElement('a');
        a.dataset.threadId = th.id;
        a.dataset.postCount = String(th.post_count || 0);
        const lastSeen = readState[th.id] || 0;
        const isRead = lastSeen >= th.post_count && th.post_count > 0;
        a.className = isRead ? 'thread-read' : 'thread-unread';
        a.textContent = th.title + ' (' + th.post_count + ' posts)';
        a.onclick = () => selectThread(th);
        li.appendChild(a);
        ul.appendChild(li);
      });
      updateThreadListReadStyles();
    }

    async function selectThread(th) {
      selectedThreadId = th.id;
      document.getElementById('current-thread-title').textContent = 'Thread: ' + th.title;
      document.getElementById('reply-section').style.display = 'block';
      document.getElementById('reply-status').textContent = '';
      await loadThreadDetail(th.id);
    }

    async function loadThreadDetail(threadId) {
      const res = await fetch('/threads/' + encodeURIComponent(threadId));
      if (!res.ok) {
        console.error('Failed to load thread detail:', res.status);
        return;
      }
      const detail = await res.json();
      const container = document.getElementById('thread-posts');
      container.innerHTML = '';
      detail.posts.forEach(post => {
        const div = document.createElement('div');
        div.className = 'post';
        const header = document.createElement('div');
        header.innerHTML = '<span class="post-title">' + (post.title || '(no title)') +
          '</span> <span class="muted">by</span> <span class="post-author">' + (post.author || 'Anonymous') + '</span>';
        const body = document.createElement('div');
        body.className = 'post-body';
        body.textContent = post.body || '';
        div.appendChild(header);
        div.appendChild(body);
        container.appendChild(div);
      });
      markThreadRead(threadId, Array.isArray(detail.posts) ? detail.posts.length : 0);
    }

    document.getElementById('new-thread-submit').addEventListener('click', async () => {
      const status = document.getElementById('new-thread-status');
      status.textContent = '';
      if (!selectedCategoryId) {
        status.textContent = 'Please select a category first.';
        return;
      }
      const titleField = document.getElementById('new-thread-title');
      const bodyField = document.getElementById('new-thread-body');
      const title = (titleField.value || '').trim();
      const body = (bodyField.value || '').trim();
      if (!title || !body) {
        status.textContent = 'Title and body are required.';
        return;
      }
      const author = getUsername();
      const res = await fetch('/threads', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          category_id: selectedCategoryId,
          title,
          author,
          body
        })
      });
      if (!res.ok) {
        const txt = await res.text();
        status.textContent = 'Failed: ' + txt;
        return;
      }
      status.textContent = 'Thread posted.';
      titleField.value = '';
      bodyField.value = '';
      await loadThreads(selectedCategoryId);
    });

    document.getElementById('reply-submit').addEventListener('click', async () => {
      const status = document.getElementById('reply-status');
      status.textContent = '';
      if (!selectedThreadId) {
        status.textContent = 'Please select a thread first.';
        return;
      }
      const bodyField = document.getElementById('reply-body');
      const body = (bodyField.value || '').trim();
      if (!body) {
        status.textContent = 'Reply body is required.';
        return;
      }
      const author = getUsername();
      const res = await fetch('/threads/' + encodeURIComponent(selectedThreadId) + '/replies', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ author, body })
      });
      if (!res.ok) {
        const txt = await res.text();
        status.textContent = 'Failed: ' + txt;
        return;
      }
      status.textContent = 'Reply posted.';
      bodyField.value = '';
      await loadThreadDetail(selectedThreadId);
    });

    // Initial load
    loadCategories().catch(console.error);
  </script>
</body>
</html>
"#
    .to_string();

    Html(body)
}

/// List all top-level categories (root child groups).
pub async fn list_categories(State(state): State<AppState>) -> impl IntoResponse {
    let db = state.db.read().await;
    let mut out = Vec::new();

    println!(
        "[/categories] root group name='{}', children={}",
        db.root.name,
        db.root.children.len()
    );

    for (idx, node) in db.root.children.iter().enumerate() {
        if let NodeRef::Group(g) = node.as_ref() {
            println!(
                "  child[{idx}] Group uuid={} name='{}'",
                g.uuid,
                g.name
            );
            out.push(CategoryDto {
                id: g.uuid.to_string(),
                name: g.name.clone(),
            });
        } else if let NodeRef::Entry(e) = node.as_ref() {
            println!(
                "  child[{idx}] Entry uuid={} title='{}'",
                e.uuid,
                e.get_title().unwrap_or("")
            );
        }
    }

    Json(out)
}

/// List all threads (child groups) in a given category.
pub async fn list_threads_in_category(
    State(state): State<AppState>,
    Path(category_id): Path<String>,
) -> impl IntoResponse {
    let db = state.db.read().await;
    println!("[GET /categories/{category_id}/threads]");
    let Some(category) = find_group_by_id(&db.root, &category_id) else {
        println!("  category not found");
        return (StatusCode::NOT_FOUND, "Category not found").into_response();
    };

    let mut out = Vec::new();
    for node in &category.children {
        if let NodeRef::Group(g) = node.as_ref() {
            let post_count = count_entries_in_group(g);
            println!(
                "  thread group uuid={} name='{}' posts={}",
                g.uuid, g.name, post_count
            );
            out.push(ThreadSummaryDto {
                id: g.uuid.to_string(),
                title: g.name.clone(),
                post_count,
            });
        }
    }

    Json(out).into_response()
}

/// Get full detail of a thread (all posts within the thread group).
pub async fn get_thread_detail(
    State(state): State<AppState>,
    Path(thread_id): Path<String>,
) -> impl IntoResponse {
    let db = state.db.read().await;
    println!("[GET /threads/{thread_id}]");
    let Some(thread_group) = find_group_by_id(&db.root, &thread_id) else {
        println!("  thread not found");
        return (StatusCode::NOT_FOUND, "Thread not found").into_response();
    };

    let mut posts = Vec::new();
    for node in &thread_group.children {
        if let NodeRef::Entry(e) = node.as_ref() {
            posts.push(entry_to_post_dto(e));
        }
    }

    let detail = ThreadDetailDto {
        id: thread_group.uuid.to_string(),
        title: thread_group.name.clone(),
        posts,
    };

    Json(detail).into_response()
}

#[derive(Deserialize)]
pub struct CreateThreadRequest {
    pub category_id: String,
    pub title: String,
    pub author: String,
    pub body: String,
}

#[derive(Deserialize)]
pub struct CreateReplyRequest {
    pub author: String,
    pub body: String,
}

/// Create a new thread in a category.
pub async fn create_thread(
    State(state): State<AppState>,
    Json(payload): Json<CreateThreadRequest>,
) -> impl IntoResponse {
    println!(
        "[POST /threads] category_id={} title='{}' author='{}'",
        payload.category_id, payload.title, payload.author
    );
    let mut db = state.db.write().await;
    let thread_id = match add_thread_to_category(
        &mut db,
        &payload.category_id,
        &payload.title,
        &payload.author,
        &payload.body,
    ) {
        Ok(id) => id,
        Err(msg) => return (StatusCode::BAD_REQUEST, msg).into_response(),
    };

    if let Err(e) = save_database(&db, &state.db_path, &state.key) {
        eprintln!("Failed to save database: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to save database",
        )
            .into_response();
    }

    (StatusCode::CREATED, thread_id).into_response()
}

/// Create a reply in an existing thread.
pub async fn create_reply(
    State(state): State<AppState>,
    Path(thread_id): Path<String>,
    Json(payload): Json<CreateReplyRequest>,
) -> impl IntoResponse {
    println!(
        "[POST /threads/{thread_id}/replies] author='{}'",
        payload.author
    );
    let mut db = state.db.write().await;
    let reply_id =
        match add_reply_to_thread(&mut db, &thread_id, &payload.author, &payload.body) {
            Ok(id) => id,
            Err(msg) => return (StatusCode::BAD_REQUEST, msg).into_response(),
        };

    if let Err(e) = save_database(&db, &state.db_path, &state.key) {
        eprintln!("Failed to save database: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to save database",
        )
            .into_response();
    }

    (StatusCode::CREATED, reply_id).into_response()
}
