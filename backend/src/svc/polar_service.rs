use std::sync::Arc;

use anyhow::Result;
use serde::Serialize;

use crate::repo::{PolarRepository, BenefitSummary};

#[derive(Clone)]
pub struct PolarService {
    repo: Arc<PolarRepository>,
}

impl PolarService {
    pub fn new(repo: Arc<PolarRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_user_limits(&self, user_id: &str) -> Result<UserLimitsResponse> {
        let state = match self
            .repo
            .get_customer_state_by_external_id(user_id)
            .await
        {
            Ok(s) => s,
            Err(_) => {
                // Polar API failing or customer not found -> return defaults
                return Ok(UserLimitsResponse {
                    has_active_subscription: false,
                    benefits: Vec::new(),
                });
            }
        };

        let has_active_sub = !state
            .active_subscriptions
            .iter()
            .filter(|s| s.status == "active")
            .collect::<Vec<_>>()
            .is_empty();

        // Fetch descriptions for each benefit by ID from Polar, fall back to local mapping
        let mut benefits = Vec::with_capacity(state.granted_benefits.len());
        for b in state.granted_benefits.into_iter() {
            let bt = b.benefit_type.clone();
            let description = match self.repo.get_benefit_by_id(&b.benefit_id).await {
                Ok(BenefitSummary { description: Some(d), .. }) => d,
                Ok(_) => bt.clone(),
                Err(_) => bt.clone(),
            };

            benefits.push(BenefitInfo {
                id: b.id,
                benefit_id: b.benefit_id,
                benefit_type: bt.clone(),
                description,
                metadata: b.benefit_metadata,
            });
        }

        Ok(UserLimitsResponse {
            has_active_subscription: has_active_sub,
            benefits,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct UserLimitsResponse {
    pub has_active_subscription: bool,
    pub benefits: Vec<BenefitInfo>,
}

#[derive(Debug, Serialize)]
pub struct BenefitInfo {
    pub id: String,
    pub benefit_id: String,
    pub benefit_type: String,
    pub description: String,
    pub metadata: serde_json::Value,
}
