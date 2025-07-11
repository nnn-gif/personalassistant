use crate::error::{AppError, Result};
use crate::models::{Activity, ProductivityInsights, ProductivityScore};
use chrono::Utc;
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;
use serde_json::json;

pub struct LlmClient {
    model_name: String,
}

impl LlmClient {
    pub fn new() -> Self {
        Self {
            model_name: "llama3.2:1b".to_string(), // Default model
        }
    }
    
    pub async fn generate_productivity_insights(&self, activities: &[Activity]) -> Result<ProductivityInsights> {
        let activities_json = serde_json::to_string_pretty(activities)?;
        
        let prompt = format!(
            "Analyze the following activity data and provide productivity insights:\n\n{}\n\n\
            Please provide:\n\
            1. A brief summary of the user's productivity patterns\n\
            2. 3-5 key insights about their work habits\n\
            3. 3-5 specific suggestions for improvement\n\n\
            Format your response as JSON with the following structure:\n\
            {{\n\
              \"summary\": \"Brief summary here\",\n\
              \"key_insights\": [\"insight1\", \"insight2\", ...],\n\
              \"suggested_improvements\": [\"improvement1\", \"improvement2\", ...]\n\
            }}",
            activities_json
        );
        
        let response = self.send_request(&prompt).await?;
        let json_response = self.extract_json(&response)?;
        
        Ok(ProductivityInsights {
            summary: json_response["summary"].as_str().unwrap_or("").to_string(),
            key_insights: json_response["key_insights"]
                .as_array()
                .map(|arr| arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect())
                .unwrap_or_default(),
            suggested_improvements: json_response["suggested_improvements"]
                .as_array()
                .map(|arr| arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect())
                .unwrap_or_default(),
            timestamp: Utc::now(),
        })
    }
    
    pub async fn generate_productivity_score(&self, activities: &[Activity]) -> Result<ProductivityScore> {
        let activities_json = serde_json::to_string_pretty(activities)?;
        
        let prompt = format!(
            "Analyze the following activity data and calculate productivity scores:\n\n{}\n\n\
            Calculate scores (0-100) for:\n\
            1. Overall productivity\n\
            2. Focus (time spent on productive tasks)\n\
            3. Efficiency (active work vs idle time)\n\
            4. Break quality (appropriate breaks taken)\n\n\
            Format your response as JSON:\n\
            {{\n\
              \"overall\": 85,\n\
              \"focus\": 90,\n\
              \"efficiency\": 80,\n\
              \"breaks\": 75\n\
            }}",
            activities_json
        );
        
        let response = self.send_request(&prompt).await?;
        let json_response = self.extract_json(&response)?;
        
        Ok(ProductivityScore {
            overall: json_response["overall"].as_f64().unwrap_or(0.0) as f32,
            focus: json_response["focus"].as_f64().unwrap_or(0.0) as f32,
            efficiency: json_response["efficiency"].as_f64().unwrap_or(0.0) as f32,
            breaks: json_response["breaks"].as_f64().unwrap_or(0.0) as f32,
            timestamp: Utc::now(),
        })
    }
    
    pub async fn generate_recommendations(&self, activities: &[Activity]) -> Result<Vec<String>> {
        let activities_json = serde_json::to_string_pretty(activities)?;
        
        let prompt = format!(
            "Based on the following activity data, provide 5 specific, actionable recommendations:\n\n{}\n\n\
            Format your response as a JSON array of strings:\n\
            [\"recommendation1\", \"recommendation2\", ...]",
            activities_json
        );
        
        let response = self.send_request(&prompt).await?;
        let json_response = self.extract_json(&response)?;
        
        Ok(json_response
            .as_array()
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect())
            .unwrap_or_default())
    }
    
    pub async fn split_research_query(&self, query: &str) -> Result<Vec<String>> {
        let prompt = format!(
            "Split the following research query into 3-5 specific subtasks that can be searched independently:\n\n\
            Query: {}\n\n\
            Format your response as a JSON array of search queries:\n\
            [\"subtask1\", \"subtask2\", ...]",
            query
        );
        
        let response = self.send_request(&prompt).await?;
        let json_response = self.extract_json(&response)?;
        
        Ok(json_response
            .as_array()
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect())
            .unwrap_or_else(|| vec![query.to_string()]))
    }
    
    pub async fn synthesize_research(&self, query: &str, results: &str) -> Result<String> {
        let prompt = format!(
            "Synthesize the following research results for the query '{}' into a comprehensive conclusion:\n\n\
            Results:\n{}\n\n\
            Provide a well-structured summary with key findings and insights.",
            query, results
        );
        
        self.send_request(&prompt).await
    }
    
    pub async fn send_request(&self, prompt: &str) -> Result<String> {
        let client = Client::default();
        
        let chat_req = ChatRequest::new(vec![
            ChatMessage::user(prompt)
        ]);
        
        let chat_response = client
            .exec_chat(&self.model_name, chat_req, None)
            .await
            .map_err(|e| AppError::Llm(format!("LLM request failed: {}", e)))?;
        
        chat_response.content_text_as_str()
            .ok_or_else(|| AppError::Llm("Empty response from LLM".into()))
            .map(|s| s.to_string())
    }
    
    fn extract_json(&self, text: &str) -> Result<serde_json::Value> {
        // Try to extract JSON from the response
        // LLMs sometimes wrap JSON in markdown code blocks
        let cleaned = if text.contains("```json") {
            text.split("```json")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(text)
        } else if text.contains("```") {
            text.split("```")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(text)
        } else {
            text
        };
        
        serde_json::from_str(cleaned.trim())
            .map_err(|e| AppError::Llm(format!("Failed to parse JSON response: {}", e)))
    }
}