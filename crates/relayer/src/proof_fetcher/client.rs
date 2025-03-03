use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use ethers::types::Bytes;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Serialize)]
struct RequestProofParams {
    jsonrpc: String,
    id: i64,
    method: String,
    params: Vec<u64>,
}

#[derive(Deserialize)]
struct RequestProofResponse {
    result: i64,
}

#[derive(Serialize)]
struct QueryProofParams {
    jsonrpc: String,
    id: i64,
    method: String,
    params: Vec<i64>,
}

#[derive(Deserialize)]
struct QueryProofResponse {
    result: QueryProofResult,
}

#[derive(Deserialize)]
struct QueryProofResult {
    #[serde(default)]
    proof: String,
    status: String,
}

pub struct ProofApiClient {
    token: String,
    endpoint: String,
}

impl ProofApiClient {
    pub fn new(token: String, endpoint: String) -> Self {
        Self { token, endpoint }
    }

    pub async fn fetch_proof(
        &self,
        chain_id: u32,
        block_number: u64,
        tx_index: u32,
        log_index: u32,
    ) -> Result<Bytes> {
        let job_id = self
            .request_proof(chain_id, block_number, tx_index, log_index)
            .await?;

        let mut attempts = 0;
        loop {
            let result = self.query_proof(job_id).await?;
            if result.status == "ready" || result.status == "complete" {
                let proof_bytes = general_purpose::STANDARD.decode(&result.proof)?;
                return Ok(Bytes::from(proof_bytes));
            }

            attempts += 1;
            if attempts > 5 {
                return Err(anyhow::anyhow!("Timeout waiting for proof"));
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    #[instrument(skip(self), fields(chain_id = chain_id, block_number = block_number, tx_index = tx_index, log_index = log_index))]
    async fn request_proof(
        &self,
        chain_id: u32,
        block_number: u64,
        tx_index: u32,
        log_index: u32,
    ) -> Result<i64> {
        let client = reqwest::Client::new();

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.token))?,
        );

        let params = RequestProofParams {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "log_requestProof".to_string(),
            params: vec![
                chain_id as u64,
                block_number,
                tx_index as u64,
                log_index as u64,
            ],
        };

        let response = client
            .post(&self.endpoint)
            .headers(headers)
            .json(&params)
            .send()
            .await?;

        let text = response.text().await?;
        tracing::info!(response = %text, method = "log_requestProof", "Raw proof response");
        let proof_response: RequestProofResponse = serde_json::from_str(&text)?;
        Ok(proof_response.result)
    }

    #[instrument(skip(self), fields(job_id = job_id))]
    async fn query_proof(&self, job_id: i64) -> Result<QueryProofResult> {
        let client = reqwest::Client::new();

        let params = QueryProofParams {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "log_queryProof".to_string(),
            params: vec![job_id],
        };

        let response = client.post(&self.endpoint).json(&params).send().await?;

        let text = response.text().await?;
        tracing::info!(response = %text, method = "log_queryProof", "Raw query response");
        let proof_response: QueryProofResponse = serde_json::from_str(&text)?;
        Ok(proof_response.result)
    }
}
