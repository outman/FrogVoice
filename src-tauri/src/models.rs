use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFile {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub duration_secs: Option<f64>,
    pub extension: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FileConvertStatus {
    Pending,
    Converting { progress: f64 },
    Completed { output_size: u64 },
    Failed { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertProgressEvent {
    pub file_path: String,
    pub progress: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertStatusEvent {
    pub file_path: String,
    pub status: FileConvertStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertDoneEvent {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
}
