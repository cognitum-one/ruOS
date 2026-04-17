//! LLM module — placeholder for future candle-based local inference.
//! Currently the agent calls the external ruos-llm-serve via HTTP.
//! This module will eventually replace it with inline candle inference.

// The LLM reasoning logic is in profile.rs (llm_reason function)
// which calls the HTTP endpoint. When we port the LLM to inline candle,
// this module will own the model loading and inference.
