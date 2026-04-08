use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileReference {
    pub object_key: String,  // "file:instance-{uuid}-{varname}-{filename}"
    pub filename: String,    // "report.pdf"
    pub mime_type: String,   // "application/pdf"
    pub size_bytes: u64,     // 1245678
    pub uploaded_at: String, // ISO 8601 timestamp
}

impl FileReference {
    /// Creates a new FileReference and generates the object_key.
    pub fn new(
        instance_id: Uuid,
        var_name: &str,
        filename: &str,
        mime_type: &str,
        size_bytes: u64,
    ) -> Self {
        let object_key = format!("file:{instance_id}-{var_name}-{filename}");
        Self {
            object_key,
            filename: filename.to_string(),
            mime_type: mime_type.to_string(),
            size_bytes,
            uploaded_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Converts this reference to a serde_json::Value for storage in variables.
    pub fn to_variable_value(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "file",
            "object_key": self.object_key,
            "filename": self.filename,
            "mime_type": self.mime_type,
            "size_bytes": self.size_bytes,
            "uploaded_at": self.uploaded_at
        })
    }

    /// Tries to parse a serde_json::Value as a FileReference.
    /// Returns None if the value doesn't have `"type": "file"`.
    pub fn from_variable_value(value: &serde_json::Value) -> Option<Self> {
        if value.get("type").and_then(|t| t.as_str()) == Some("file") {
            serde_json::from_value(value.clone()).ok()
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Process definition (validated at construction time)
// ---------------------------------------------------------------------------

