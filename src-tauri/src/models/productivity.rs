use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductivityScore {
    pub overall: f32,
    pub focus: f32,
    pub efficiency: f32,
    pub breaks: f32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductivityInsights {
    pub summary: String,
    pub key_insights: Vec<String>,
    pub suggested_improvements: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    pub chart_type: ChartType,
    pub data_points: Vec<DataPoint>,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChartType {
    Line,
    Bar,
    Pie,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub label: String,
    pub value: f32,
    pub color: Option<String>,
}
