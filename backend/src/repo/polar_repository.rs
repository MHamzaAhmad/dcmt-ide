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

    // List products (optionally filter for recurring plans)
    pub async fn list_products(&self, is_recurring: Option<bool>) -> Result<Vec<Product>> {
        if self.access_token.is_empty() {
            return Err(anyhow!("POLAR_ACCESS_TOKEN not configured"));
        }
        let mut url = format!("{}/v1/products/", self.base_url);
        if let Some(rec) = is_recurring {
            url.push_str(&format!("?is_recurring={}", rec));
        }
        let res = self
            .http
            .get(url)
            .bearer_auth(&self.access_token)
            .header(reqwest::header::USER_AGENT, "dcmt-polar-client/1.0")
            .send()
            .await?;

        match res.status() {
            StatusCode::OK => {
                let list = res.json::<ListResource<Product>>().await?;
                Ok(list.items)
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(anyhow!("Unauthorized: invalid POLAR_ACCESS_TOKEN or scope"))
            }
            s => Err(anyhow!("Polar API error: {} - {}", s, res.text().await.unwrap_or_default())),
        }
    }

    // Create checkout session for a product (simplified)
    pub async fn create_checkout_session(&self, product_id: &str, success_url: &str, customer_external_id: &str) -> Result<CheckoutSessionResponse> {
        if self.access_token.is_empty() {
            return Err(anyhow!("POLAR_ACCESS_TOKEN not configured"));
        }
        let url = format!("{}/v1/checkout/sessions", self.base_url);
        let payload = serde_json::json!({
            "product_id": product_id,
            "success_url": success_url,
            "customer_external_id": customer_external_id,
        });
        let res = self
            .http
            .post(url)
            .bearer_auth(&self.access_token)
            .header(reqwest::header::USER_AGENT, "dcmt-polar-client/1.0")
            .json(&payload)
            .send()
            .await?;

        match res.status() {
            StatusCode::OK | StatusCode::CREATED => {
                let out = res.json::<CheckoutSessionResponse>().await?;
                Ok(out)
            }
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

// Products list wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResource<T> {
    pub items: Vec<T>,
    pub pagination: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductPrice {
    pub id: String,
    pub amount_type: Option<String>,
    pub price_amount: Option<i64>,
    pub price_currency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub is_recurring: bool,
    pub description: Option<String>,
    pub prices: Vec<ProductPrice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutSessionResponse {
    pub id: String,
    pub url: String,
}
