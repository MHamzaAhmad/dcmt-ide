import type { BillingOperations, LimitsResponse } from '../../types';

// Mock desktop billing adapter for now
export class DesktopBillingAdapter implements BillingOperations {
	async getLimits(): Promise<LimitsResponse> {
		// Return a safe default with no subscription and no benefits
		return {
			has_active_subscription: false,
			benefits: []
		};
	}
}
