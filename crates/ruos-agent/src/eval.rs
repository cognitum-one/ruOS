//! Self-evaluation: test search quality, detect drift.

use crate::{http_post_json, log, write_state, BRAIN_URL};
use chrono::Local;
use serde_json::{json, Value};

pub async fn self_eval() -> Value {
    let queries = vec![
        "GPU optimization",
        "system profile management",
        "contrastive learning",
        "brain memory storage",
    ];

    let mut results = Vec::new();
    for query in &queries {
        let resp = http_post_json(
            &format!("{BRAIN_URL}/brain/search"),
            &json!({"query": query, "k": 3}),
        ).await;

        match resp {
            Some(data) => {
                let hits = data.get("results").and_then(|v| v.as_array());
                let top_score = hits.and_then(|h| h.first())
                    .and_then(|r| r.get("score"))
                    .and_then(|s| s.as_f64())
                    .unwrap_or(0.0);
                let has_content = hits.map(|h| h.iter().any(|r| r.get("content").is_some())).unwrap_or(false);
                results.push(json!({
                    "query": query,
                    "hits": hits.map(|h| h.len()).unwrap_or(0),
                    "top_score": (top_score * 10000.0).round() / 10000.0,
                    "has_content": has_content,
                }));
            }
            None => {
                results.push(json!({"query": query, "status": "search_failed"}));
            }
        }
    }

    let scores: Vec<f64> = results.iter()
        .filter_map(|r| r.get("top_score").and_then(|v| v.as_f64()))
        .collect();
    let avg = if scores.is_empty() { 0.0 } else { scores.iter().sum::<f64>() / scores.len() as f64 };
    let avg_rounded = (avg * 10000.0).round() / 10000.0;

    if avg < 0.3 {
        log("WARN", &format!("SELF-EVAL: degraded — avg score {avg_rounded}"));
    } else {
        log("INFO", &format!("SELF-EVAL: OK — avg score {avg_rounded}"));
    }

    let eval_result = json!({
        "at": Local::now().to_rfc3339(),
        "queries": results.len(),
        "avg_top_score": avg_rounded,
        "details": results,
    });
    write_state("last_eval", &eval_result);
    eval_result
}
