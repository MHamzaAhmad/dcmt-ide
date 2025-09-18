import { apiClient } from '../../client';
import type { BillingOperations, LimitsResponse } from '../../types';

export class WebBillingAdapter implements BillingOperations {
	async getLimits(): Promise<LimitsResponse> {
		return apiClient.get<LimitsResponse>('/api/billing/limits');
	}
}
