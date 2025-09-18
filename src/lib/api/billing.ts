// Billing API Endpoints (Unified platform-based)
import type { LimitsResponse } from './types';
import { getBillingAdapter } from './adapters';

export const billingAPI = {
	async getLimits(): Promise<LimitsResponse> {
		return getBillingAdapter().getLimits();
	},
	async createCheckoutSession(): Promise<{ url: string }> {
		return getBillingAdapter().createCheckoutSession();
	}
};
