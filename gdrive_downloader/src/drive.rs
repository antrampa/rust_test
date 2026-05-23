// ============================================================
//  src/drive.rs
//  Wraps the Google Drive v3 REST API for listing files.
// ============================================================

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

// ── Response types ──

#[derive(Debug, Deserialize)]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    // Only present for binary files (not Google Docs/Sheets/etc.)
    pub size: Option<String>,
    #[serde(rename = "modifiedTime")]
    pub modified_time: Option<String>,
}


#[derive(Deserialize)]
struct FileListResponse {
    files: Vec<DriveFile>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
} 


// ── Client ──

pub struct DriveClient {
    pub http: Client,
    pub token: Option<String>,
}

impl DriveClient {
    pub fn new(token: Option<String>) -> Result<Self> {
        let http = Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .context("Failed to build HTTP Client")?;
        Ok(Self {http, token})
    } 

    /// List all files inside `folder_id`.
    /// If `folder_id` is None, lists files in the root of My Drive.
    /// Automatically pages through all results.
    pub async fn list_files(&self, folder_id: Option<&str>) -> Result<Vec<DriveFile>> {
        const BASE: &str = "https://www.googleapis.com/drive/v3/files";

        // Build the `q` query filter
        let q = match folder_id {
            Some(id) => format!("'{}' in parents and trashed = false", id),
            None => "trashed = false".into(),    
        };

        // Fields we want back for each file
        let fields = "nextPageToken,files(id,name,mimeType,size,modifiedTime)";

        let mut all_files: Vec<DriveFile> = Vec::new();
        let mut page_token: Option<String> = None;
        
        loop {
            let mut req = self
                    .http
                    .get(BASE)
                    .query(&[
                        ("q", q.as_str()),
                        ("fields", fields),
                        ("pageSize", "100"),
                        ("orderBy", "name"),
                    ]);

            // Auth header for private files
            if let Some(token) = &self.token {
                req = req.bearer_auth(token);
            }

            // Pagination
            if let Some(ref pt) = page_token {
                req = req.query(&[("pageToken", pt.as_str())]);
            }

            let resp = req.send().await.context("Drive list request failed")?;
            let status = resp.status();

            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                anyhow::bail!("Drive API error {status}: {body}");
            }

            let page: FileListResponse = resp.json().await.context("Failed to parse file list")?;
            all_files.extend(page.files);

            match page.next_page_token {
                Some(pt) => page_token = Some(pt),
                None => break,
            }
        }

        Ok(all_files)
    }
} 

