#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CreatedIssueDto {
    pub id: String,
    pub key: String,
    pub self_url: Option<String>,
}

impl CreatedIssueDto {
    pub fn new(id: String, key: String, self_url: Option<String>) -> Self {
        Self { id, key, self_url }
    }
}
