use axum::{
    extract::Path, response::Html, routing::get, Json, Router 
};
use chrono::Datelike;
use std::sync::{Mutex, MutexGuard};

static WORD: Mutex<String> = Mutex::new(String::new());
static LAST_WORD_CHANGE: Mutex<i32> = Mutex::new(0);

#[tokio::main]
async fn main() {
    println!("=== WordleAPI_rs Starting | Version {} ===", env!("CARGO_PKG_VERSION"));

    println!("Router starting...");
    let app: Router = Router::new()
        .route("/check/{word}", get(check_word))
        .route("/{*path}", get(get_html))
        .route("/", get(get_index));
    println!("Router started");

    println!("Setting initial word...");
    if !set_word() {
        println!("Error setting initial word. Please check the words.txt file.");
        return;
    }
    println!("Initial word set");

    println!("Starting listener...");
    let listener = tokio::net::TcpListener::bind("localhost:8080").await.unwrap();

    println!("Starting server...");
    axum::serve(listener, app).await.unwrap();
}

const CONTENT_DIR: &str = "http_doc/";
const INDEX_HTML: &str = "index.html";
const BLANK_PATH: &str = "/";

const ERR_404: &str = "http_doc/404.html";

async fn get_html(Path(path): Path<String>) -> Html<String> {
    println!("Request received for html");
    // Normalize the path to prevent directory traversal attacks
    let path = path.trim_start_matches('/').to_string();

    if !path.ends_with(".html") {
        println!("Error - Path does not end with .html");
        return Html("Error 400 - Bad Request".to_string());
    }

    // Handle for index, which does not need a path
    if path.is_empty() || path == BLANK_PATH {
        println!("Responded with index.html");
        return Html(std::fs::read_to_string(CONTENT_DIR.to_string() + INDEX_HTML).unwrap_or_else(|_| "Error 500 - Please contact owner".to_string()));
    }

    return match std::fs::read_to_string(CONTENT_DIR.to_string() + &path) {
        Ok(content) => {
            println!("Responded with {}", path);
            Html(content)
        }
        Err(_) => {
            println!("Error reading {}", path);
            return match std::fs::read_to_string(ERR_404) {
                Ok(content) => Html(content),
                // If everything goes wrong, return a 500 error
                Err(_) => Html("Error 500 - Please contact owner".to_string()),
            };
        }
    };
}

async fn get_index() -> Html<String> {
    println!("Request received for index");
    return match std::fs::read_to_string(CONTENT_DIR.to_string() + INDEX_HTML) {
        Ok(content) => {
            println!("Responded with {}", INDEX_HTML);
            Html(content)
        }
        Err(_) => {
            println!("Error reading {}", INDEX_HTML);
            return match std::fs::read_to_string(ERR_404) {
                Ok(content) => Html(content),
                // If everything goes wrong, return a 500 error
                Err(_) => Html("Error 500 - Please contact owner".to_string()),
            };
        }
    };
}

/// Take a word, and session ID and check which letters are correct
/// Returns JSON format with the result
/// Error codes:
/// - Error - 1: Word not found in words.txt
/// - Error - 2: System error
async fn check_word(Path(word): Path<String>) -> Json<String> {
    println!("Request received");

    let last_word_change = LAST_WORD_CHANGE.lock().unwrap();
    let current_day = chrono::Utc::now().day() as i32;
    if *last_word_change != current_day {
        println!("Word has changed, setting new word");
        if !set_word() {
            println!("Error setting new word. Please check the words.txt file.");
            return Json("{\"Error\":\"2\"}".to_string());
        }
    }

    let words = match std::fs::read_to_string(WORD_FILE) {
        Ok(x) => x,
        Err(_) => return Json("{\"Error\":\"2\"}".to_string()),
    };

    let words: Vec<&str> = words.lines().collect();
    if !words.contains(&word.as_str()) {
        println!("Error - Word not found in words.txt");
        return Json("{\"Error\":\"1\"}".to_string());
    }

    let word_lock: MutexGuard<String> = WORD.lock().unwrap();
    let answer: &str = word_lock.as_str();
    if word_lock.is_empty() {
        println!("Error - No word set");
        return Json("{\"Error\":\"2\"}".to_string());
    }

    if answer == &word {
        println!("Responded");
        return Json("{\"Error\":\"0\",\"L1\":2,\"L2\":2,\"L3\":2,\"L4\":2,\"L5\":2}".to_string());
    }

    let mut result = String::from("{\"Error\":\"2\",");
    let answer_chars: Vec<char> = answer.chars().collect();
    for (i, c) in word.chars().enumerate() {
        if c == answer_chars[i] {
            result.push_str(&format!("\"L{}\":2,", i + 1));
        } else if answer_chars.contains(&c) {
            result.push_str(&format!("\"L{}\":1,", i + 1));
        } else {
            result.push_str(&format!("\"L{}\":0,", i + 1));
        }
    }
    result.pop();
    result = result + "}";
    println!("Responded");
    return Json(result);
}

const WORD_FILE: &str = "words.txt";

/// Randomly choose a word from a file, then put it into the WORD mutex
fn set_word() -> bool {
    // Load the file and choose a random word
    let words = match std::fs::read_to_string(WORD_FILE) {
        Ok(x) => x,
        Err(_) => return false,
    };

    let words: Vec<&str> = words.lines().collect();
    if words.is_empty() {
        return false;
    }
    let random_index = rand::random::<u32>() % words.len() as u32;
    let random_word = words[random_index as usize];

    let mut word = WORD.lock().unwrap();
    *word = random_word.into();

    if cfg!(debug_assertions) {
        println!("Word set to: {}", random_word);
    }

    let mut last_word_change = LAST_WORD_CHANGE.lock().unwrap();
    *last_word_change = chrono::Utc::now().day() as i32;

    true
}