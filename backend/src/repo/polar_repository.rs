use anyhow::{anyhow, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct PolarRepository {
    http: Client,
    base_url: String,
    access_token: String,
}

impl PolarRepository {
    pub fn new(access_token: String, sandbox: bool) -> Result<Self> {
        Ok(Self {
            http: Client::new(),
            base_url: if sandbox {
                "https://sandbox-api.polar.sh".to_string()
            } else {
                "https://api.polar.sh".to_string()
            },
            access_token,
        })
    }

    pub async fn get_customer_state_by_external_id(&self, external_id: &str) -> Result<CustomerState> {
        if self.access_token.is_empty() {
            return Err(anyhow!("POLAR_ACCESS_TOKEN not configured"));
        }
        let url = format!(
            "{}/v1/customers/external/{}/state",
            self.base_url,
            urlencoding::encode(external_id)
        );

        let res = self
            .http
            .get(url)
            .bearer_auth(&self.access_token)
            .header(reqwest::header::USER_AGENT, "dcmt-polar-client/1.0")
            .send()
            .await?;

        match res.status() {
            StatusCode::OK => {
                let state = res.json::<CustomerState>().await?;
                Ok(state)
            }
            StatusCode::NOT_FOUND => Err(anyhow!("Customer not found")),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(anyhow!("Unauthorized: invalid POLAR_ACCESS_TOKEN or scope"))
            }
            s => Err(anyhow!("Polar API error: {} - {}", s, res.text().await.unwrap_or_default())),
        }
    }

    pub async fn get_benefit_by_id(&self, id: &str) -> Result<BenefitSummary> {
        if self.access_token.is_empty() {
            return Err(anyhow!("POLAR_ACCESS_TOKEN not configured"));
        }
        let url = format!(
            "{}/v1/benefits/{}",
            self.base_url,
            urlencoding::encode(id)
        );

        let res = self
            .http
            .get(url)
            .bearer_auth(&self.access_token)
            .header(reqwest::header::USER_AGENT, "dcmt-polar-client/1.0")
            .send()
            .await?;

        match res.status() {
            StatusCode::OK => {
                let b = res.json::<BenefitSummary>().await?;
                Ok(b)
            }
            StatusCode::NOT_FOUND => Err(anyhow!("Benefit not found")),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(anyhow!("Unauthorized: invalid POLAR_ACCESS_TOKEN or scope"))
            }
            s => Err(anyhow!("Polar API error: {} - {}", s, res.text().await.unwrap_or_default())),
        }
    }
}

// Schemas (partial: include only what we need for limits/benefits & subscription status)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerState {
    pub id: String,
    pub external_id: Option<String>,
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
    pub active_subscriptions: Vec<CustomerStateSubscription>,
    pub granted_benefits: Vec<CustomerStateBenefitGrant>,
    pub active_meters: Vec<CustomerStateMeter>,
    pub avatar_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerStateSubscription {
    pub id: String,
    pub status: String,
    pub amount: Option<i64>,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerStateBenefitGrant {
    pub id: String,
    pub benefit_id: String,
    pub benefit_type: String,
    #[serde(default)]
    pub benefit_metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerStateMeter {
    pub id: String,
    pub consumed_units: f64,
    pub credited_units: i64,
    pub balance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenefitSummary {
    pub id: String,
    #[serde(rename = "type")]
    pub benefit_type: String,
    pub description: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}
