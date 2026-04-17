//! Embed backfill: find memories without vectors, embed them.

use crate::{log, EMBEDDER_URL};
use rusqlite::Connection;
use serde_json::json;
use std::path::PathBuf;

fn db_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join("brain-data/brain.sqlite")
}

fn blob_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join("brain-data/blobs")
}

fn blob_read(hash: &str) -> Option<String> {
    if hash.len() < 4 { return None; }
    let path = blob_dir().join(&hash[..2]).join(&hash[2..]);
    std::fs::read_to_string(path).ok()
}

pub async fn embed_backfill(max_batch: usize) -> usize {
    let conn = match Connection::open(db_path()) {
        Ok(c) => c,
        Err(_) => return 0,
    };

    let mut stmt = match conn.prepare(
        "SELECT hex(id), content_hash FROM memories WHERE LENGTH(embedding) = 0 OR embedding IS NULL LIMIT ?1"
    ) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let rows: Vec<(String, String)> = match stmt.query_map([max_batch as i64], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) {
        Ok(mapped) => mapped.filter_map(|r| r.ok()).collect(),
        Err(_) => return 0,
    };
    drop(stmt);

    if rows.is_empty() {
        log("INFO", "BACKFILL: all memories have embeddings");
        return 0;
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();

    let mut embedded = 0;
    for (id_hex, content_hash) in &rows {
        let content = match blob_read(content_hash) {
            Some(c) if c.len() >= 3 => c,
            _ => continue,
        };

        // Get embedding
        let resp: serde_json::Value = match client
            .post(format!("{EMBEDDER_URL}/embed"))
            .json(&json!({"texts": [content]}))
            .send().await
        {
            Ok(r) => match r.json().await {
                Ok(v) => v,
                Err(_) => continue,
            },
            Err(_) => continue,
        };

        let vectors = resp.get("vectors").or(resp.get("embeddings"))
            .and_then(|v| v.as_array());
        let vec = match vectors.and_then(|a| a.first()).and_then(|v| v.as_array()) {
            Some(v) => v,
            None => continue,
        };

        // Write embedding to SQLite
        let emb_blob: Vec<u8> = vec.iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .flat_map(|f| f.to_le_bytes())
            .collect();

        if let Ok(id_blob) = hex::decode(id_hex) {
            let _ = conn.execute(
                "UPDATE memories SET embedding = ?1 WHERE id = ?2",
                rusqlite::params![emb_blob, id_blob],
            );
            embedded += 1;
        }
    }

    if embedded > 0 {
        log("INFO", &format!("BACKFILL: embedded {embedded}/{} memories", rows.len()));
        if embedded > 10 {
            let _ = std::process::Command::new("systemctl")
                .args(["--user", "restart", "ruvultra-brain.service"])
                .output();
            log("INFO", "BACKFILL: restarted brain to rebuild index");
        }
    }

    embedded
}
